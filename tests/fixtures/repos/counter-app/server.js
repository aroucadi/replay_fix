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

const server = http.createServer((req, res) => {
  const reqUrl = (req.url || "/").split("?")[0];
  let filePath = path.join(__dirname, reqUrl === "/" ? "index.html" : reqUrl);
  
  if (!fs.existsSync(filePath)) {
    filePath = path.join(__dirname, "index.html");
  }

  const ext = path.extname(filePath).toLowerCase();
  const mimeTypes = {
    ".html": "text/html; charset=utf-8",
    ".js": "application/javascript; charset=utf-8",
    ".ts": "application/javascript; charset=utf-8",
    ".tsx": "application/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".json": "application/json; charset=utf-8",
  };
  const contentType = mimeTypes[ext] || "text/plain; charset=utf-8";

  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
      res.end("Not Found");
      return;
    }
    res.writeHead(200, { "Content-Type": contentType });
    res.end(data);
  });
});

const requestedPort = resolveRequestedPort();
server.listen(requestedPort, "127.0.0.1", () => {
  const address = server.address();
  const boundPort = address && typeof address === "object" ? address.port : requestedPort;
  console.log(`[counter-app] HTTP server ready at http://127.0.0.1:${boundPort}`);
});
