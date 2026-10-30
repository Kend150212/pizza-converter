# Security & Anti-Tamper Policy

**Pizza Converter** by **Ken Dao / NEEFLOW**  
Version: **0.1.0**

---

## 1. Zero-Tolerance Anti-Tamper & Anti-Reversing Policy

Pizza Converter is proprietary intellectual property. We enforce active security measures to protect the integrity of our software, intellectual property, and user security:

1. **Anti-Debugging & Dynamic Analysis Protection:**
   - Active detection mechanisms (`IsDebuggerPresent`, `CheckRemoteDebuggerPresent`, memory hooking detection) are embedded directly within the native compiled binary.
   - Any attempt to attach a debugger, inspect process memory, or inject foreign dynamic link libraries (DLL injection / API hooking) triggers an immediate security alert and emergency process shutdown.

2. **Binary Obfuscation & Strip Hardening:**
   - Production binaries are built with Link-Time Optimization (`lto = "fat"`), single-codegen unit emission (`codegen-units = 1`), highest optimization (`opt-level = 3`), and full symbol stripping (`strip = true`).
   - Debug symbols, internal structure definitions, and function maps are purged to prevent reverse engineering and unauthorized disassembly.

3. **Canary Traps & Honeypot Tokens (Anti-Bot / Anti-Cracking):**
   - The application incorporates canary honeypot signatures to trap automated vulnerability scanners, bot agents, and license-cracking tools.
   - Any agent, bot, or automated tool attempting to bypass security boundaries, extract proprietary keys, or manipulate runtime licenses will trigger an immediate revocation of the associated license key and machine authorization.

4. **100% On-Device Processing Guarantee:**
   - The security model is strictly verified to ensure zero telemetry and zero external transmission of user files, documents, photos, or conversion data.

---

## 2. Reporting Security Vulnerabilities

We take security and user safety seriously. If you discover a legitimate vulnerability, please report it responsibly:

- **Security Team Email:** security@neeflow.com
- **Response SLA:** Within 48 hours for acknowledged reports.

Please provide detailed replication steps and allow a reasonable embargo period before any public disclosure.
