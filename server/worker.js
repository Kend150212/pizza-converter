/**
 * Pizza Converter - Cloudflare Worker License Server
 */
const ADMIN_SECRET = "KenDaoPizza2026";

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization",
    };

    if (request.method === "OPTIONS") return new Response(null, { headers: corsHeaders });

    if (url.pathname === "/api/redeem" && request.method === "POST") {
      try {
        const { code, machine_id } = await request.json();
        if (!code || !machine_id) {
          return new Response(JSON.stringify({ success: false, error: "Missing code or machine_id" }), {
            status: 400, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const normalizedCode = code.trim().toUpperCase();
        const blacklisted = await env.PIZZA_KV.get(`blacklist:${machine_id}`);
        if (blacklisted) {
          return new Response(JSON.stringify({ success: false, error: "This device is permanently blacklisted." }), {
            status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const recordJson = await env.PIZZA_KV.get(`code:${normalizedCode}`);
        if (!recordJson) {
          return new Response(JSON.stringify({ success: false, error: "Invalid redeem code." }), {
            status: 404, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const record = JSON.parse(recordJson);
        if (record.status !== "active") {
          return new Response(JSON.stringify({ success: false, error: `Code is ${record.status}.` }), {
            status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        if (!record.devices) record.devices = [];
        if (!record.devices.includes(machine_id)) {
          if (record.devices.length >= (record.max_devices || 1)) {
            return new Response(JSON.stringify({ success: false, error: "Device limit reached for this code." }), {
              status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }
          record.devices.push(machine_id);
          record.activations = record.devices.length;
          await env.PIZZA_KV.put(`code:${normalizedCode}`, JSON.stringify(record));
        }
        return new Response(JSON.stringify({
          success: true, tier: record.tier || "pro",
          devices_used: record.devices.length, max_devices: record.max_devices || 1,
          message: "Pro license activated successfully!"
        }), { status: 200, headers: { ...corsHeaders, "Content-Type": "application/json" } });
      } catch (err) {
        return new Response(JSON.stringify({ success: false, error: err.message }), {
          status: 500, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }
    }

    if (url.pathname === "/api/tamper" && request.method === "POST") {
      try {
        const { code, machine_id, reason } = await request.json();
        if (machine_id) {
          await env.PIZZA_KV.put(`blacklist:${machine_id}`, JSON.stringify({
            blacklisted_at: new Date().toISOString(), reason: reason || "Debugger / reverse engineering detected"
          }));
        }
        if (code) {
          const normalizedCode = code.trim().toUpperCase();
          const recordJson = await env.PIZZA_KV.get(`code:${normalizedCode}`);
          if (recordJson) {
            const record = JSON.parse(recordJson);
            record.status = "revoked";
            record.revoked_reason = reason || "Security tampering";
            record.revoked_at = new Date().toISOString();
            await env.PIZZA_KV.put(`code:${normalizedCode}`, JSON.stringify(record));
          }
        }
        return new Response(JSON.stringify({ success: true, message: "Tamper report logged. License revoked." }), {
          status: 200, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      } catch (err) {
        return new Response(JSON.stringify({ success: false, error: err.message }), {
          status: 500, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }
    }

    if (url.pathname === "/admin") {
      const cookies = request.headers.get("Cookie") || "";
      const hasSession = cookies.includes("pizza_admin_session=authenticated_ken_dao_secure");
      const queryKey = url.searchParams.get("key");

      if (request.method === "POST" && url.searchParams.get("login") === "true") {
        const formData = await request.formData();
        const enteredPassword = formData.get("password");
        if (enteredPassword === ADMIN_SECRET) {
          return new Response(null, {
            status: 302,
            headers: {
              "Location": "/admin",
              "Set-Cookie": "pizza_admin_session=authenticated_ken_dao_secure; Path=/admin; HttpOnly; Secure; SameSite=Strict; Max-Age=86400"
            }
          });
        } else {
          return new Response(`
            <!DOCTYPE html><html><head><meta charset="utf-8"><title>Access Denied</title>
            <style>body{font-family:sans-serif;background:#090d16;color:#f87171;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;}</style>
            </head><body><div style="background:#131c2e;padding:30px;border-radius:12px;border:1px solid #ef4444;text-align:center;">
              <h2>Wrong Password</h2><p>Access denied.</p><br><a href="/admin" style="color:#5eead4;">Try Again</a>
            </div></body></html>
          `, { status: 401, headers: { "Content-Type": "text/html" } });
        }
      }

      if (url.searchParams.get("logout") === "true") {
        return new Response(null, {
          status: 302,
          headers: {
            "Location": "/admin",
            "Set-Cookie": "pizza_admin_session=; Path=/admin; HttpOnly; Secure; SameSite=Strict; Max-Age=0"
          }
        });
      }

      const isAuthenticated = hasSession || (queryKey === ADMIN_SECRET);
      if (!isAuthenticated) {
        return new Response(`
          <!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1.0"><title>Pizza Converter Admin - Login</title>
          <style>body{font-family:-apple-system,sans-serif;background:#090d16;color:#fff;display:flex;justify-content:center;align-items:center;height:100vh;margin:0;}
          .card{background:#131c2e;border:1px solid #1e293b;padding:36px 30px;border-radius:16px;width:320px;text-align:center;box-shadow:0 12px 36px rgba(0,0,0,0.5);}
          input{width:100%;padding:12px;margin:16px 0;border-radius:10px;border:1px solid #334155;background:#090d16;color:#fff;box-sizing:border-box;font-size:14px;}
          button{width:100%;padding:12px;border-radius:10px;border:none;background:#2dd4bf;color:#0f172a;font-weight:700;cursor:pointer;font-size:14px;}
          </style></head>
          <body>
            <div class="card">
              <div style="font-size:42px;margin-bottom:8px;">🍕</div>
              <h2 style="margin:0 0 6px 0;color:#5eead4;">Restricted Area</h2>
              <p style="font-size:13px;color:#94a3b8;margin:0 0 16px 0;">Enter master admin password</p>
              <form method="POST" action="/admin?login=true">
                <input type="password" name="password" placeholder="Admin password..." required autofocus />
                <button type="submit">Unlock Dashboard</button>
              </form>
            </div>
          </body></html>
        `, { headers: { "Content-Type": "text/html" } });
      }

      if (request.method === "POST") {
        const formData = await request.formData();
        const action = formData.get("action");
        if (action === "generate") {
          const count = parseInt(formData.get("count") || "5", 10);
          const maxDev = parseInt(formData.get("max_devices") || "1", 10);
          const tier = formData.get("tier") || "lifetime";
          for (let i = 0; i < count; i++) {
            const randPart = Array.from(crypto.getRandomValues(new Uint8Array(6)))
              .map(b => b.toString(36).toUpperCase()).join("").slice(0, 8);
            const newCode = `PIZZA-${randPart.slice(0, 4)}-${randPart.slice(4, 8)}`;
            const record = {
              code: newCode, tier: tier, max_devices: maxDev,
              devices: [], activations: 0, status: "active", created_at: new Date().toISOString()
            };
            await env.PIZZA_KV.put(`code:${newCode}`, JSON.stringify(record));
          }
        }
        if (action === "revoke") {
          const targetCode = formData.get("code")?.trim().toUpperCase();
          if (targetCode) {
            const rJson = await env.PIZZA_KV.get(`code:${targetCode}`);
            if (rJson) {
              const r = JSON.parse(rJson);
              r.status = "revoked";
              r.revoked_at = new Date().toISOString();
              await env.PIZZA_KV.put(`code:${targetCode}`, JSON.stringify(r));
            }
          }
        }
      }

      const list = await env.PIZZA_KV.list({ prefix: "code:" });
      const codes = [];
      for (const k of list.keys) {
        const val = await env.PIZZA_KV.get(k.name);
        if (val) codes.push(JSON.parse(val));
      }
      codes.sort((a, b) => new Date(b.created_at) - new Date(a.created_at));

      return new Response(`
        <!DOCTYPE html><html><head><meta charset="utf-8"><title>Pizza Converter Admin</title>
        <style>
          body{font-family:-apple-system,sans-serif;background:#090d16;color:#e2e8f0;padding:30px;max-width:960px;margin:0 auto;}
          h1{color:#5eead4;margin-bottom:6px;display:inline-block;}
          .top-bar{display:flex;justify-content:space-between;align-items:center;margin-bottom:20px;}
          .card{background:#131c2e;border:1px solid #1e293b;border-radius:14px;padding:20px;margin-bottom:24px;}
          table{width:100%;border-collapse:collapse;margin-top:14px;}
          th,td{padding:10px 12px;text-align:left;border-bottom:1px solid #1e293b;font-size:13px;}
          th{color:#94a3b8;font-weight:600;}
          .badge{display:inline-block;padding:3px 8px;border-radius:999px;font-size:11px;font-weight:700;}
          .badge-active{background:rgba(45,212,191,0.15);color:#2dd4bf;border:1px solid rgba(45,212,191,0.3);}
          .badge-revoked{background:rgba(239,68,68,0.15);color:#f87171;border:1px solid rgba(239,68,68,0.3);}
          button{padding:8px 16px;border-radius:8px;border:none;background:#2dd4bf;color:#0f172a;font-weight:700;cursor:pointer;}
          .btn-danger{background:#ef4444;color:#fff;}
          .btn-logout{background:#334155;color:#f87171;padding:8px 14px;border-radius:8px;text-decoration:none;font-size:13px;font-weight:700;}
          input,select{padding:8px 12px;border-radius:8px;border:1px solid #334155;background:#090d16;color:#fff;}
        </style></head>
        <body>
          <div class="top-bar">
            <div>
              <h1>🍕 Pizza Converter - License Dashboard</h1>
              <p style="color:#94a3b8;margin:4px 0 0 0;">Manage Redeem Codes, activations, and security blacklists.</p>
            </div>
            <a href="/admin?logout=true" class="btn-logout">Log Out</a>
          </div>
          <div class="card">
            <h3>Generate New Redeem Codes</h3>
            <form method="POST" action="/admin" style="display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin-top:12px;">
              <input type="hidden" name="action" value="generate" />
              <label>Amount: <input type="number" name="count" value="5" min="1" max="100" style="width:60px;" /></label>
              <label>Max Devices: <input type="number" name="max_devices" value="1" min="1" max="10" style="width:50px;" /></label>
              <label>Tier:
                <select name="tier">
                  <option value="lifetime">Lifetime Pro</option>
                  <option value="1-year">1 Year Pro</option>
                </select>
              </label>
              <button type="submit">+ Create Codes</button>
            </form>
          </div>
          <div class="card">
            <h3>Active & Generated Codes (${codes.length})</h3>
            <table>
              <thead>
                <tr><th>Redeem Code</th><th>Tier</th><th>Devices</th><th>Status</th><th>Created At</th><th>Action</th></tr>
              </thead>
              <tbody>
                ${codes.map(c => `
                  <tr>
                    <td><strong style="letter-spacing:1px;font-family:monospace;">${c.code}</strong></td>
                    <td>${c.tier}</td>
                    <td>${(c.devices || []).length} / ${c.max_devices || 1}</td>
                    <td><span class="badge ${c.status === 'active' ? 'badge-active' : 'badge-revoked'}">${c.status.toUpperCase()}</span></td>
                    <td style="color:#94a3b8;">${new Date(c.created_at).toLocaleString()}</td>
                    <td>
                      ${c.status === 'active' ? `
                        <form method="POST" action="/admin" style="display:inline;">
                          <input type="hidden" name="action" value="revoke" />
                          <input type="hidden" name="code" value="${c.code}" />
                          <button type="submit" class="btn-danger" style="padding:4px 10px;font-size:11px;">Revoke</button>
                        </form>
                      ` : '<span style="color:#64748b;">Revoked</span>'}
                    </td>
                  </tr>
                `).join('')}
              </tbody>
            </table>
          </div>
        </body></html>
      `, { headers: { "Content-Type": "text/html" } });
    }

    return new Response("Pizza Converter License Server is Online.", { status: 200 });
  }
};
