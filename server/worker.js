/**
 * Pizza Converter - Cloudflare Worker License Server (Enterprise Grade)
 * Full-featured Dashboard: Bulk Generation, Multi-Select, Bulk Copy, Revoke, Resolve, Export
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

    // API: Validate & Redeem Code
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
          return new Response(JSON.stringify({ success: false, error: "This device is permanently blacklisted due to security tampering." }), {
            status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const recordJson = await env.PIZZA_KV.get(`code:${normalizedCode}`);
        if (!recordJson) {
          return new Response(JSON.stringify({ success: false, error: "Invalid redeem code. Please check your purchase." }), {
            status: 404, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        const record = JSON.parse(recordJson);
        if (record.status !== "active") {
          return new Response(JSON.stringify({ success: false, error: `This code has been revoked or expired (${record.status}).` }), {
            status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
          });
        }
        if (!record.devices) record.devices = [];
        if (!record.devices.includes(machine_id)) {
          if (record.devices.length >= (record.max_devices || 1)) {
            return new Response(JSON.stringify({ success: false, error: "Maximum activation limit reached for this code." }), {
              status: 403, headers: { ...corsHeaders, "Content-Type": "application/json" }
            });
          }
          record.devices.push(machine_id);
          record.activations = record.devices.length;
          await env.PIZZA_KV.put(`code:${normalizedCode}`, JSON.stringify(record));
        }
        return new Response(JSON.stringify({
          success: true,
          tier: record.tier || "pro",
          code: normalizedCode,
          devices_used: record.devices.length,
          max_devices: record.max_devices || 1,
          message: "Pro license activated successfully!"
        }), { status: 200, headers: { ...corsHeaders, "Content-Type": "application/json" } });
      } catch (err) {
        return new Response(JSON.stringify({ success: false, error: err.message }), {
          status: 500, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }
    }

    // API: Tamper & Anti-Crack Report
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
        return new Response(JSON.stringify({ success: true, message: "Tamper logged. License revoked." }), {
          status: 200, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      } catch (err) {
        return new Response(JSON.stringify({ success: false, error: err.message }), {
          status: 500, headers: { ...corsHeaders, "Content-Type": "application/json" }
        });
      }
    }

    // ADMIN DASHBOARD
    if (url.pathname === "/admin") {
      const cookies = request.headers.get("Cookie") || "";
      const hasSession = cookies.includes("pizza_admin_session=authenticated_ken_dao_secure");
      const queryKey = url.searchParams.get("key");

      // Handle Login
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

      // Handle Logout
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

      // Handle Admin Actions (Generate, Bulk Revoke, Bulk Resolve, Bulk Delete)
      if (request.method === "POST") {
        const formData = await request.formData();
        const action = formData.get("action");

        // Generate Multiple Codes
        if (action === "generate") {
          const count = Math.min(Math.max(parseInt(formData.get("count") || "5", 10), 1), 200);
          const maxDev = parseInt(formData.get("max_devices") || "1", 10);
          const tier = formData.get("tier") || "lifetime";
          for (let i = 0; i < count; i++) {
            const randPart = Array.from(crypto.getRandomValues(new Uint8Array(8)))
              .map(b => b.toString(36).toUpperCase()).join("").slice(0, 8);
            const newCode = `PIZZA-${randPart.slice(0, 4)}-${randPart.slice(4, 8)}`;
            const record = {
              code: newCode,
              tier: tier,
              max_devices: maxDev,
              devices: [],
              activations: 0,
              status: "active",
              created_at: new Date().toISOString()
            };
            await env.PIZZA_KV.put(`code:${newCode}`, JSON.stringify(record));
          }
        }

        // Single Revoke
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

        // Single Resolve / Re-activate
        if (action === "resolve") {
          const targetCode = formData.get("code")?.trim().toUpperCase();
          if (targetCode) {
            const rJson = await env.PIZZA_KV.get(`code:${targetCode}`);
            if (rJson) {
              const r = JSON.parse(rJson);
              r.status = "active";
              delete r.revoked_at;
              delete r.revoked_reason;
              await env.PIZZA_KV.put(`code:${targetCode}`, JSON.stringify(r));
            }
          }
        }

        // Bulk Actions
        if (action === "bulk_action") {
          const bulkOp = formData.get("bulk_op");
          const selectedCodes = formData.getAll("selected_codes");
          for (const c of selectedCodes) {
            const targetCode = c.trim().toUpperCase();
            if (bulkOp === "revoke") {
              const rJson = await env.PIZZA_KV.get(`code:${targetCode}`);
              if (rJson) {
                const r = JSON.parse(rJson);
                r.status = "revoked";
                r.revoked_at = new Date().toISOString();
                await env.PIZZA_KV.put(`code:${targetCode}`, JSON.stringify(r));
              }
            } else if (bulkOp === "resolve") {
              const rJson = await env.PIZZA_KV.get(`code:${targetCode}`);
              if (rJson) {
                const r = JSON.parse(rJson);
                r.status = "active";
                delete r.revoked_at;
                delete r.revoked_reason;
                await env.PIZZA_KV.put(`code:${targetCode}`, JSON.stringify(r));
              }
            } else if (bulkOp === "delete") {
              await env.PIZZA_KV.delete(`code:${targetCode}`);
            }
          }
        }
      }

      // Fetch all codes
      const list = await env.PIZZA_KV.list({ prefix: "code:" });
      const codes = [];
      for (const k of list.keys) {
        const val = await env.PIZZA_KV.get(k.name);
        if (val) codes.push(JSON.parse(val));
      }
      codes.sort((a, b) => new Date(b.created_at) - new Date(a.created_at));

      return new Response(`
        <!DOCTYPE html><html><head><meta charset="utf-8">
        <title>Pizza Converter Pro - Admin Dashboard</title>
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <style>
          * { box-sizing: border-box; }
          body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #0b0f19; color: #e2e8f0; margin: 0; padding: 24px 32px; }
          .top-bar { display: flex; justify-content: space-between; align-items: center; padding-bottom: 20px; border-bottom: 1px solid #1e293b; margin-bottom: 24px; }
          h1 { margin: 0; color: #5eead4; font-size: 24px; font-weight: 800; display: flex; align-items: center; gap: 8px; }
          .btn-logout { background: rgba(239,68,68,0.15); color: #f87171; border: 1px solid rgba(239,68,68,0.3); padding: 8px 16px; border-radius: 8px; text-decoration: none; font-size: 13px; font-weight: 700; transition: all .15s; }
          .btn-logout:hover { background: #ef4444; color: #fff; }
          .grid { display: grid; grid-template-columns: 1fr; gap: 20px; }
          .card { background: #131c2e; border: 1px solid #1e293b; border-radius: 14px; padding: 20px 24px; }
          .card h3 { margin: 0 0 14px 0; font-size: 16px; color: #f1f5f9; display: flex; justify-content: space-between; align-items: center; }
          
          /* Form controls */
          input, select { padding: 9px 12px; border-radius: 8px; border: 1px solid #334155; background: #090d16; color: #fff; font-size: 13px; }
          input:focus, select:focus { outline: none; border-color: #2dd4bf; }
          button { padding: 9px 18px; border-radius: 8px; border: none; background: #2dd4bf; color: #090d16; font-weight: 700; font-size: 13px; cursor: pointer; transition: all .15s; }
          button:hover { filter: brightness(1.1); }
          .btn-secondary { background: #1e293b; color: #cbd5e1; border: 1px solid #334155; }
          .btn-secondary:hover { background: #334155; color: #fff; }
          .btn-danger { background: #ef4444; color: #fff; }
          .btn-success { background: #10b981; color: #fff; }
          
          /* Toolbar */
          .toolbar { display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 12px; margin-bottom: 14px; background: rgba(0,0,0,0.25); padding: 12px 16px; border-radius: 10px; border: 1px solid #1e293b; }
          .bulk-actions { display: flex; align-items: center; gap: 8px; }
          
          /* Table */
          table { width: 100%; border-collapse: collapse; margin-top: 8px; }
          th, td { padding: 12px 14px; text-align: left; border-bottom: 1px solid #1e293b; font-size: 13px; }
          th { color: #94a3b8; font-weight: 700; background: rgba(0,0,0,0.15); }
          tr:hover td { background: rgba(255,255,255,0.02); }
          .code-cell { font-family: monospace; font-size: 14px; font-weight: 700; color: #5eead4; letter-spacing: 0.05em; }
          .badge { display: inline-block; padding: 3px 9px; border-radius: 999px; font-size: 11px; font-weight: 700; letter-spacing: 0.03em; }
          .badge-active { background: rgba(45,212,191,0.15); color: #2dd4bf; border: 1px solid rgba(45,212,191,0.35); }
          .badge-revoked { background: rgba(239,68,68,0.15); color: #f87171; border: 1px solid rgba(239,68,68,0.35); }
          
          .btn-sm { padding: 4px 10px; font-size: 11px; border-radius: 6px; }
          .toast { position: fixed; bottom: 24px; right: 24px; background: #10b981; color: #fff; padding: 12px 20px; border-radius: 8px; font-weight: 700; display: none; z-index: 9999; box-shadow: 0 8px 24px rgba(0,0,0,0.4); }
        </style>
        </head>
        <body>
          <div id="toast" class="toast">✓ Copied to clipboard!</div>
          <div class="top-bar">
            <div>
              <h1>🍕 Pizza Converter — License Dashboard</h1>
              <p style="color:#94a3b8;margin:6px 0 0 0;font-size:13.5px;">Master Code Generation, Real-Time Hardware Activations & Blacklist Control</p>
            </div>
            <a href="/admin?logout=true" class="btn-logout">Sign Out</a>
          </div>

          <div class="grid">
            <!-- Code Generator Form -->
            <div class="card">
              <h3>⚡ Generate New Redeem Codes</h3>
              <form method="POST" action="/admin" style="display:flex;gap:14px;align-items:center;flex-wrap:wrap;">
                <input type="hidden" name="action" value="generate" />
                <label style="font-size:13px;font-weight:600;">Amount: 
                  <input type="number" name="count" value="20" min="1" max="100" style="width:80px;font-weight:700;" />
                </label>
                <label style="font-size:13px;font-weight:600;">Max Devices: 
                  <input type="number" name="max_devices" value="1" min="1" max="10" style="width:65px;font-weight:700;" />
                </label>
                <label style="font-size:13px;font-weight:600;">Tier:
                  <select name="tier" style="font-weight:600;">
                    <option value="lifetime">Lifetime Pro ($19.99)</option>
                    <option value="1-year">1 Year Pro</option>
                  </select>
                </label>
                <button type="submit">+ Generate Batch Codes</button>
                <button type="button" class="btn-secondary" onclick="quickGen(20)">Quick 20 Codes</button>
                <button type="button" class="btn-secondary" onclick="quickGen(50)">Quick 50 Codes</button>
              </form>
            </div>

            <!-- Codes Management Table with Multi-Select -->
            <div class="card">
              <h3>
                <span>Redeem Codes (${codes.length})</span>
                <span style="font-size:12px;color:#94a3b8;font-weight:normal;">Manage activations & license revocation</span>
              </h3>

              <form id="bulkForm" method="POST" action="/admin">
                <input type="hidden" name="action" value="bulk_action" />
                <input type="hidden" name="bulk_op" id="bulkOpInput" value="" />

                <div class="toolbar">
                  <div class="bulk-actions">
                    <button type="button" class="btn-secondary" onclick="copySelected()">📋 Copy Selected</button>
                    <button type="button" class="btn-secondary" onclick="exportSelected()">📥 Export Selected (TXT)</button>
                    <button type="button" class="btn-danger" onclick="submitBulk('revoke')">🚫 Revoke</button>
                    <button type="button" class="btn-success" onclick="submitBulk('resolve')">✅ Resolve (Re-activate)</button>
                    <button type="button" class="btn-secondary" style="color:#f87171;" onclick="submitBulk('delete')">🗑️ Delete</button>
                    <span id="selectedCount" style="font-size:12.5px;color:#94a3b8;margin-left:8px;">0 selected</span>
                  </div>
                  <div>
                    <input type="text" id="searchInput" placeholder="Search code or device..." onkeyup="filterTable()" style="width:220px;" />
                  </div>
                </div>

                <table id="codesTable">
                  <thead>
                    <tr>
                      <th style="width:36px;"><input type="checkbox" id="selectAll" onclick="toggleSelectAll(this)" /></th>
                      <th>Redeem Code</th>
                      <th>Tier</th>
                      <th>Devices Bound</th>
                      <th>Status</th>
                      <th>Created Date</th>
                      <th style="text-align:right;">Quick Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    ${codes.map(c => `
                      <tr data-code="${c.code}" data-status="${c.status}">
                        <td><input type="checkbox" name="selected_codes" value="${c.code}" class="code-check" onchange="updateCount()" /></td>
                        <td class="code-cell">${c.code}</td>
                        <td style="text-transform:capitalize;">${c.tier}</td>
                        <td>
                          <strong>${(c.devices || []).length} / ${c.max_devices || 1}</strong>
                          ${(c.devices && c.devices.length > 0) ? `<br><small style="color:#64748b;font-family:monospace;font-size:10px;">${c.devices.map(d=>d.slice(0,10)+'...').join(', ')}</small>` : ''}
                        </td>
                        <td>
                          <span class="badge ${c.status === 'active' ? 'badge-active' : 'badge-revoked'}">
                            ${c.status.toUpperCase()}
                          </span>
                        </td>
                        <td style="color:#94a3b8;font-size:12px;">${new Date(c.created_at).toLocaleString()}</td>
                        <td style="text-align:right;">
                          <button type="button" class="btn-secondary btn-sm" onclick="copySingle('${c.code}')">Copy</button>
                          ${c.status === 'active' ? `
                            <button type="button" class="btn-danger btn-sm" onclick="quickAction('${c.code}', 'revoke')">Revoke</button>
                          ` : `
                            <button type="button" class="btn-success btn-sm" onclick="quickAction('${c.code}', 'resolve')">Resolve</button>
                          `}
                        </td>
                      </tr>
                    `).join("")}
                  </tbody>
                </table>
              </form>
            </div>
          </div>

          <form id="singleActionForm" method="POST" action="/admin" style="display:none;">
            <input type="hidden" name="action" id="singleAction" />
            <input type="hidden" name="code" id="singleCode" />
          </form>

          <script>
            function showToast(msg) {
              const t = document.getElementById('toast');
              t.textContent = msg || '✓ Copied to clipboard!';
              t.style.display = 'block';
              setTimeout(() => { t.style.display = 'none'; }, 2200);
            }

            function quickGen(num) {
              const input = document.querySelector('input[name="count"]');
              if (input) {
                input.value = num;
                input.form.submit();
              }
            }

            function toggleSelectAll(master) {
              const checks = document.querySelectorAll('.code-check');
              checks.forEach(c => {
                if (c.closest('tr').style.display !== 'none') {
                  c.checked = master.checked;
                }
              });
              updateCount();
            }

            function updateCount() {
              const checked = document.querySelectorAll('.code-check:checked');
              document.getElementById('selectedCount').textContent = checked.length + ' selected';
            }

            function getSelectedCodes() {
              const checked = document.querySelectorAll('.code-check:checked');
              return Array.from(checked).map(c => c.value);
            }

            function copySingle(code) {
              navigator.clipboard.writeText(code).then(() => showToast('✓ Copied ' + code));
            }

            function copySelected() {
              const codes = getSelectedCodes();
              if (codes.length === 0) { alert('Please select at least one code to copy.'); return; }
              navigator.clipboard.writeText(codes.join('\\n')).then(() => showToast('✓ Copied ' + codes.length + ' codes!'));
            }

            function exportSelected() {
              const codes = getSelectedCodes();
              if (codes.length === 0) { alert('Please select codes to export.'); return; }
              const blob = new Blob([codes.join('\\n')], { type: 'text/plain;charset=utf-8' });
              const a = document.createElement('a');
              a.href = URL.createObjectURL(blob);
              a.download = 'pizza_converter_codes_' + Date.now() + '.txt';
              a.click();
            }

            function submitBulk(op) {
              const codes = getSelectedCodes();
              if (codes.length === 0) { alert('Please select at least one code.'); return; }
              const actionLabel = op === 'revoke' ? 'REVOKE' : op === 'resolve' ? 'RESOLVE / RE-ACTIVATE' : 'DELETE PERMANENTLY';
              if (!confirm('Are you sure you want to ' + actionLabel + ' ' + codes.length + ' selected codes?')) return;
              document.getElementById('bulkOpInput').value = op;
              document.getElementById('bulkForm').submit();
            }

            function quickAction(code, action) {
              if (action === 'revoke' && !confirm('Revoke license code ' + code + '?')) return;
              document.getElementById('singleAction').value = action;
              document.getElementById('singleCode').value = code;
              document.getElementById('singleActionForm').submit();
            }

            function filterTable() {
              const q = document.getElementById('searchInput').value.toLowerCase();
              const rows = document.querySelectorAll('#codesTable tbody tr');
              rows.forEach(r => {
                const text = r.textContent.toLowerCase();
                r.style.display = text.includes(q) ? '' : 'none';
              });
            }
          </script>
        </body></html>
      `, { headers: { "Content-Type": "text/html; charset=utf-8" } });
    }

    return new Response("Pizza Converter License Server is Online.", { status: 200 });
  }
};
