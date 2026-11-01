const http = require("node:http");
const fs = require("node:fs");
const path = require("node:path");

function resolveRequestedPort() {
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i += 1) {
    if ((args[i] === "--port" || args[i] === "-p") && args[i + 1] !== undefined) {
      const parsed = Number.parseInt(args[i + 1], 10);
      if (!Number.isNaN(parsed) && parsed >= 0 && parsed <= 65535) {
        return parsed;
      }
    }
  }
  if (process.env.PORT !== undefined) {
    const envPort = Number.parseInt(process.env.PORT, 10);
    if (!Number.isNaN(envPort) && envPort >= 0 && envPort <= 65535) {
      return envPort;
    }
  }
  return 0;
}

const publicDir = path.join(__dirname, "public");

const server = http.createServer((req, res) => {
  const reqUrl = (req.url || "/").split("?")[0];

  if (reqUrl === "/api/health") {
    const payload = JSON.stringify({ status: "ok", service: "metapulse" });
    res.writeHead(200, {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "no-store",
    });
    res.end(payload);
    return;
  }

  let filePath;
  if (reqUrl === "/" || reqUrl === "/index.html" || reqUrl === "/public/index.html") {
    filePath = path.join(publicDir, "index.html");
  } else if (reqUrl === "/public/app.js" || reqUrl === "/app.js") {
    filePath = path.join(publicDir, "app.js");
  } else {
    res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Not Found");
    return;
  }

  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      res.end("Not Found");
      return;
    }
    const contentType = filePath.endsWith(".js")
      ? "application/javascript; charset=utf-8"
      : "text/html; charset=utf-8";
    res.writeHead(200, {
      "Content-Type": contentType,
      "Cache-Control": "no-store",
    });
    res.end(data);
  });
});

const requestedPort = resolveRequestedPort();
server.listen(requestedPort, "127.0.0.1", () => {
  const addr = server.address();
  const actualPort = typeof addr === "object" && addr ? addr.port : requestedPort;
  console.log(`MetaPulse server listening on http://127.0.0.1:${actualPort}`);
});
