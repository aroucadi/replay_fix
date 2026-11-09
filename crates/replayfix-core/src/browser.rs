//! Embedded sandbox HTTP server and Node.js Playwright stdio JSON-lines bridge.
//!
//! `SandboxServer` serves files from an isolated `ScratchSandbox` over
//! `127.0.0.1:<ephemeral_port>` with strict path validation and non-blocking
//! connection handling.
//!
//! `PlaywrightBridge` spawns `node` running the embedded bridge script and
//! communicates via flushed, line-delimited JSON with monotonic `request_id`
//! correlation, background `stdout`/`stderr` reader threads, strict per-command
//! deadlines, and idempotent process-tree teardown.

use crate::error::{CoreError, Result};
use crate::locator::DomCandidate;
use crate::manifest::TaskManifest;
use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Filename used when materializing the bridge script inside a scratch sandbox.
pub const BRIDGE_SCRIPT_FILENAME: &str = ".replayfix-playwright-bridge.cjs";

/// Default timeout for launching Node.js + Chromium (`START_BROWSER`).
pub const DEFAULT_BRIDGE_STARTUP_TIMEOUT: Duration = Duration::from_secs(25);

/// Default timeout for individual bridge commands (`PROBE_PAGE`, `CLICK_AND_OBSERVE`).
pub const DEFAULT_BRIDGE_COMMAND_TIMEOUT: Duration = Duration::from_secs(25);

/// Bounded timeout for graceful `SHUTDOWN` IPC before forced process-tree termination.
const SHUTDOWN_COMMAND_TIMEOUT: Duration = Duration::from_millis(1500);

/// Bounded poll deadline when reaping the child process handle during shutdown.
const CHILD_REAP_TIMEOUT: Duration = Duration::from_millis(500);

/// Maximum bytes accepted for a single JSON-line on bridge stdout (4 MiB) to prevent OOM DoS.
const MAX_STDOUT_LINE_BYTES: usize = 4 * 1024 * 1024;

/// Maximum number of recent stderr lines retained for diagnostic reporting.
const MAX_STDERR_LINES: usize = 64;

/// Maximum single static file size served by `SandboxServer` (32 MiB).
const MAX_HTTP_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Embedded Node.js Playwright bridge speaking line-delimited JSON over stdio.
pub const PLAYWRIGHT_BRIDGE_JS: &str = r#"const fs = require('fs');
const path = require('path');
const readline = require('readline');
const { execSync } = require('child_process');

function findPlaywrightInNpxCache(cacheDir) {
  if (!cacheDir || !fs.existsSync(cacheDir)) return null;
  const npxDir = path.join(cacheDir, '_npx');
  if (!fs.existsSync(npxDir)) return null;
  const entries = fs.readdirSync(npxDir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isDirectory()) {
      const candidate = path.join(npxDir, entry.name, 'node_modules', 'playwright');
      if (fs.existsSync(candidate)) {
        return candidate;
      }
    }
  }
  return null;
}

function loadPlaywright() {
  const fastCandidates = [];
  if (process.env.REPLAYFIX_PLAYWRIGHT_PATH) {
    fastCandidates.push(process.env.REPLAYFIX_PLAYWRIGHT_PATH);
  }
  fastCandidates.push('playwright');
  try {
    const parentNodeModules = path.resolve(__dirname, '..', 'node_modules', 'playwright');
    fastCandidates.push(parentNodeModules);
    const cwdNodeModules = path.resolve(process.cwd(), 'node_modules', 'playwright');
    fastCandidates.push(cwdNodeModules);
  } catch (_) {}

  if (process.env.LOCALAPPDATA) {
    const localCache = findPlaywrightInNpxCache(path.join(process.env.LOCALAPPDATA, 'npm-cache'));
    if (localCache) fastCandidates.push(localCache);
  }
  fastCandidates.push('C:/nvm4w/nodejs/node_modules/playwright');

  for (const target of fastCandidates) {
    try {
      return require(target);
    } catch (_) {}
  }

  const slowCandidates = [];
  try {
    const cacheOut = execSync(
      process.platform === 'win32' ? 'npm.cmd config get cache' : 'npm config get cache',
      { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 4000 }
    ).trim();
    const fromCache = findPlaywrightInNpxCache(cacheOut);
    if (fromCache) slowCandidates.push(fromCache);
  } catch (_) {}

  try {
    const globalRoot = execSync(process.platform === 'win32' ? 'npm.cmd root -g' : 'npm root -g', { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 3000 }).trim();
    if (globalRoot) slowCandidates.push(path.join(globalRoot, 'playwright'));
  } catch (_) {}

  for (const target of slowCandidates) {
    try {
      return require(target);
    } catch (_) {}
  }
  throw new Error("Playwright module not found. Install via 'npm install -D playwright' and 'npx playwright install chromium'.");
}

function withTimeout(promise, ms, label) {
  let timer = null;
  const timeoutPromise = new Promise((_, reject) => {
    timer = setTimeout(() => {
      reject(new Error(label + ' timed out after ' + ms + 'ms'));
    }, ms);
  });
  return Promise.race([promise, timeoutPromise]).finally(() => {
    if (timer) clearTimeout(timer);
  });
}

function sendLine(obj, requestId) {
  const payload = Object.assign({}, obj);
  if (typeof requestId === 'number') {
    payload.request_id = requestId;
  }
  process.stdout.write(JSON.stringify(payload) + '\n');
}

(async () => {
  let pw;
  let browser = null;
  let browserConnected = false;
  let page = null;
  let pageCrashed = false;
  let pageErrors = [];
  let activeShadowPath = null;
  let activeShadowBody = null;
  let networkLatencyMs = 0;
  let networkJitterMs = 0;

  async function settleDomAndNetwork(activePage) {
    await Promise.race([
      activePage.waitForLoadState('networkidle', { timeout: 2000 }).catch(() => {}),
      new Promise((resolve) => setTimeout(resolve, 2000)),
    ]);
    await activePage
      .evaluate(
        () =>
          new Promise((resolve) => {
            let quietTimer = setTimeout(done, 400);
            const maxCap = setTimeout(done, 2500);
            const target = document.documentElement || document.body;
            if (!target) {
              clearTimeout(quietTimer);
              clearTimeout(maxCap);
              resolve();
              return;
            }
            const obs = new MutationObserver(() => {
              clearTimeout(quietTimer);
              quietTimer = setTimeout(done, 400);
            });
            obs.observe(target, { childList: true, subtree: true, attributes: true });
            function done() {
              clearTimeout(quietTimer);
              clearTimeout(maxCap);
              obs.disconnect();
              resolve();
            }
          })
      )
      .catch(() => {});
  }

  function ensureActivePage(cmdName) {
    if (!browser || !browserConnected) {
      throw new Error('Browser WebSocket disconnected before ' + cmdName);
    }
    if (pageCrashed) {
      throw new Error('Browser target page crashed before ' + cmdName);
    }
    if (!page) {
      throw new Error('Browser page not initialized or already closed before ' + cmdName);
    }
  }

  const rl = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });

  for await (const rawLine of rl) {
    const line = rawLine.trim();
    if (!line) continue;

    let cmd;
    try {
      cmd = JSON.parse(line);
    } catch (err) {
      sendLine({ kind: 'ERROR', message: 'Invalid JSON command: ' + String(err.message || err) }, undefined);
      continue;
    }

    const reqId = typeof cmd.request_id === 'number' ? cmd.request_id : undefined;

    try {
      if (cmd.cmd === 'START_BROWSER') {
        if (!pw) pw = loadPlaywright();
        if (browser) {
          await withTimeout(browser.close(), 2000, 'browser.close').catch(() => {});
          browser = null;
          browserConnected = false;
          page = null;
        }
        try {
          browser = await pw.chromium.launch({ headless: !cmd.headed, timeout: 15000 });
        } catch (launchErr) {
          const msg = String(launchErr && launchErr.message ? launchErr.message : launchErr);
          if (msg.includes("Executable doesn't exist")) {
            try {
              browser = await pw.chromium.launch({ channel: 'chrome', headless: !cmd.headed, timeout: 15000 });
            } catch (_) {
              browser = await pw.chromium.launch({ channel: 'msedge', headless: !cmd.headed, timeout: 15000 });
            }
          } else {
            throw launchErr;
          }
        }
        browserConnected = true;
        pageCrashed = false;
        browser.on('disconnected', () => {
          browserConnected = false;
          page = null;
          browser = null;
        });
        const context = await browser.newContext({
          viewport: {
            width: cmd.viewport_width || 1280,
            height: cmd.viewport_height || 720,
          },
        });
        context.setDefaultTimeout(15000);
        context.setDefaultNavigationTimeout(15000);
        await context.route('**/*', async (route) => {
          try {
            if (networkLatencyMs > 0 || networkJitterMs > 0) {
              const jitter = networkJitterMs > 0 ? Math.floor(Math.random() * networkJitterMs) : 0;
              await new Promise((r) => setTimeout(r, networkLatencyMs + jitter));
            }
            const reqUrl = new URL(route.request().url());
            const strippedShadow = activeShadowPath
              ? activeShadowPath.replace(/^(public|dist|build|static)\//i, '')
              : '';
            const shadowBase = activeShadowPath
              ? activeShadowPath.split('/').pop()
              : '';
            if (
              activeShadowPath &&
              typeof activeShadowBody === 'string' &&
              (reqUrl.pathname === '/' + activeShadowPath ||
                reqUrl.pathname.endsWith('/' + activeShadowPath) ||
                (strippedShadow &&
                  (reqUrl.pathname === '/' + strippedShadow ||
                    reqUrl.pathname.endsWith('/' + strippedShadow))) ||
                (shadowBase && reqUrl.pathname === '/' + shadowBase))
            ) {
              const isCss = activeShadowPath.toLowerCase().endsWith('.css');
              await route.fulfill({
                status: 200,
                contentType: isCss ? 'text/css; charset=utf-8' : 'application/javascript; charset=utf-8',
                headers: { 'Cache-Control': 'no-store' },
                body: activeShadowBody,
              });
              return;
            }
          } catch (_) {}
          await route.continue();
        });
        page = await context.newPage();
        pageErrors = [];
        page.on('pageerror', (err) => {
          pageErrors.push(String(err && err.stack ? err.stack : err));
        });
        page.on('crash', () => {
          pageCrashed = true;
        });
        page.on('close', () => {
          page = null;
        });
        sendLine({ kind: 'READY' }, reqId);
      } else if (cmd.cmd === 'PROBE_PAGE') {
        ensureActivePage('PROBE_PAGE');
        pageErrors = [];
        activeShadowPath = cmd.shadow_script_path ? String(cmd.shadow_script_path).replace(/^\/+/, '') : null;
        activeShadowBody = typeof cmd.shadow_script_body === 'string' ? cmd.shadow_script_body : null;
        await page.goto(cmd.url, { waitUntil: 'domcontentloaded', timeout: 10000 });
        if (cmd.reload_page) {
          await page.reload({ waitUntil: 'domcontentloaded', timeout: 10000 });
        }
        await settleDomAndNetwork(page);
        ensureActivePage('PROBE_PAGE(post-goto)');
        await page.evaluate(() => {
          const handles = document.querySelectorAll('[class*="handle"]');
          handles.forEach((el) => {
            if (!el.getAttribute('role')) el.setAttribute('role', 'slider');
            if (!el.getAttribute('aria-label')) {
              const cls = typeof el.className === 'string' ? el.className : '';
              const m = cls.match(/handle-([a-z]+)/i);
              const name = m ? 'Resize Handle ' + m[1].toUpperCase() : 'Resize Handle';
              el.setAttribute('aria-label', name);
            }
          });
        }).catch(() => {});
        const ariaSnapshot = await withTimeout(
          page.locator('body').ariaSnapshot({ timeout: 10000 }),
          12000,
          'ariaSnapshot'
        );
        const candidates = await withTimeout(
          page.evaluate(({ x, y }) => {
            const selector = 'button, a, input, select, textarea, nav, [role="button"], [role="link"], [role="checkbox"], [role="menuitem"], [role="slider"], [role="separator"], [role="navigation"], [class*="handle"], [class*="nav"], [class*="menu"], [data-testid]';
            const hitEl = document.elementFromPoint(x, y);
            const closestHit = hitEl ? hitEl.closest(selector) : null;
            const allInteractive = Array.from(document.querySelectorAll(selector));
            if (closestHit && !allInteractive.includes(closestHit)) {
              allInteractive.unshift(closestHit);
            }

            function inferRole(el) {
              const explicit = (el.getAttribute('role') || '').trim();
              if (explicit) return explicit.toLowerCase();
              const tag = el.tagName.toLowerCase();
              if (tag === 'button') return 'button';
              if (tag === 'a' && el.hasAttribute('href')) return 'link';
              if (tag === 'nav') return 'navigation';
              if (tag === 'input') {
                const t = (el.getAttribute('type') || 'text').toLowerCase();
                if (t === 'button' || t === 'submit' || t === 'reset') return 'button';
                if (t === 'checkbox') return 'checkbox';
                return 'textbox';
              }
              return tag;
            }

            function inferAccessibleName(el) {
              const ariaLabel = (el.getAttribute('aria-label') || '').trim();
              if (ariaLabel) return ariaLabel;
              const placeholder = (el.getAttribute('placeholder') || '').trim();
              if (placeholder) return placeholder;
              const title = (el.getAttribute('title') || '').trim();
              if (title) return title;
              const text = (el.innerText || el.textContent || '').trim();
              if (text) return text;
              const val = (el.getAttribute('value') || '').trim();
              if (val) return val;
              const href = (el.getAttribute('href') || '').trim();
              if (href) return href;
              const id = (el.getAttribute('id') || '').trim();
              if (id) return '#' + id;
              const cls = typeof el.className === 'string' ? el.className.trim() : '';
              return cls;
            }

            const rawCandidates = allInteractive.map((el) => {
              const rect = el.getBoundingClientRect();
              const rawTestId = el.getAttribute('data-testid');
              const rawId = el.getAttribute('id');
              const testId = rawTestId && rawTestId.trim()
                ? rawTestId.trim()
                : (rawId && rawId.trim() ? '#' + rawId.trim() : null);
              return {
                tag: el.tagName.toLowerCase(),
                role: inferRole(el),
                accessible_name: inferAccessibleName(el),
                test_id: testId,
                bbox: {
                  x: Math.round(rect.x),
                  y: Math.round(rect.y),
                  width: Math.max(0, Math.round(rect.width)),
                  height: Math.max(0, Math.round(rect.height)),
                },
                is_element_from_point: el === closestHit,
              };
            });

            // When a feed renders repeated cards with identical (role, accessible_name),
            // keep only the spatially closest instance to (x, y) so grounding resolves the clicked card.
            const bestByRoleName = new Map();
            for (const c of rawCandidates) {
              const key = c.role + '::' + c.accessible_name;
              const cx = c.bbox.x + c.bbox.width / 2;
              const cy = c.bbox.y + c.bbox.height / 2;
              const dist = Math.hypot(cx - x, cy - y) - (c.is_element_from_point ? 10000 : 0);
              const prev = bestByRoleName.get(key);
              if (!prev || dist < prev.dist) {
                bestByRoleName.set(key, { cand: c, dist });
              }
            }
            return Array.from(bestByRoleName.values()).map((v) => v.cand);
          }, { x: cmd.click_x, y: cmd.click_y }),
          5000,
          'DOM candidate evaluation'
        );

        sendLine({
          kind: 'PROBE_RESULT',
          navigated: true,
          aria_snapshot: ariaSnapshot,
          candidates: candidates,
          page_errors: pageErrors.slice(),
        }, reqId);
      } else if (cmd.cmd === 'CLICK_AND_OBSERVE') {
        ensureActivePage('CLICK_AND_OBSERVE');
        pageErrors = [];
        let locator = null;
        if (cmd.test_id && String(cmd.test_id).startsWith('#')) {
          locator = page.locator(String(cmd.test_id)).first();
        } else if (cmd.role && cmd.accessible_name) {
          locator = page.getByRole(cmd.role, { name: cmd.accessible_name, exact: true });
        } else if (cmd.test_id) {
          locator = page.getByTestId(cmd.test_id);
        } else if (cmd.selector && String(cmd.selector).trim().length > 0) {
          locator = page.locator(cmd.selector).first();
        }

        if (locator) {
          const matchCount = await withTimeout(locator.count(), 8000, 'Locator count check');
          if (matchCount === 0) {
            throw new Error('DOM_NODE_DETACHED: Target DOM node detached or disappeared before click (count=0)');
          }
          if (matchCount > 1 && cmd.selector === '__AMBIGUOUS__') {
            throw new Error('AMBIGUOUS_GROUNDING: Target locator matched ' + matchCount + ' DOM nodes at click time (strict uniqueness violated)');
          }
          const activeLoc = matchCount > 1 ? locator.first() : locator;
          const box = await withTimeout(activeLoc.boundingBox({ timeout: 8000 }), 9000, 'Locator boundingBox check');
          if (!box || box.width <= 0 || box.height <= 0) {
            throw new Error('DOM_NODE_DETACHED: Target DOM node became hidden or zero-sized before click');
          }
          await activeLoc.click({ timeout: 20000 });
          if (cmd.role === 'textbox') {
            const isSearch = String(cmd.accessible_name || '').toLowerCase().includes('search')
              || String(cmd.test_id || '').toLowerCase().includes('search');
            const sampleInput = isSearch ? 'query[?*' : 'Testing verified diagnostic input payload for validation check.';
            await activeLoc.fill(sampleInput).catch(() => {});
            if (!isSearch) {
              const submitBtn = page.locator('button[type="submit"], button:has-text("Post"), button:has-text("Submit"), button:has-text("Publish")').first();
              if ((await submitBtn.count()) > 0 && (await submitBtn.isEnabled().catch(() => false))) {
                await submitBtn.click({ timeout: 5000 }).catch(() => {});
              }
            }
          } else if (cmd.role === 'button') {
            const btnName = String(cmd.accessible_name || '').toLowerCase();
            const btnId = String(cmd.test_id || '').toLowerCase();
            if (btnName.includes('post') || btnName.includes('publish') || btnId.includes('submit')) {
              const composeInput = page.locator('textarea, input[type="text"]').first();
              if ((await composeInput.count()) > 0) {
                const curVal = await composeInput.inputValue().catch(() => '');
                if (!curVal) {
                  await composeInput.fill('Testing verified diagnostic input payload for validation check.').catch(() => {});
                  if (await activeLoc.isEnabled().catch(() => false)) {
                    await activeLoc.click({ timeout: 5000 }).catch(() => {});
                  }
                }
              }
            } else if (btnName.includes('quote') || btnId.includes('repost')) {
              await page.waitForTimeout(150);
              const dialogBtn = page.locator('[role="dialog"] button, .modal button, button:has-text("Publish")').last();
              if ((await dialogBtn.count()) > 0 && (await dialogBtn.isVisible().catch(() => false))) {
                await dialogBtn.click({ timeout: 5000 }).catch(() => {});
              }
            }
          } else if (cmd.role === 'combobox' || cmd.role === 'select') {
            await page.evaluate(() => {
              const sel = document.querySelector('select');
              if (sel && sel.options.length > 1) {
                sel.selectedIndex = 1;
                sel.dispatchEvent(new Event('change', { bubbles: true }));
              }
            }).catch(() => {});
          }
        } else if (typeof cmd.click_x === 'number' && typeof cmd.click_y === 'number') {
          await page.mouse.click(cmd.click_x, cmd.click_y);
        } else {
          throw new Error('Grounded click requires (role + accessible_name), test_id, selector, or (click_x, click_y)');
        }

        await page.waitForTimeout(150);
        ensureActivePage('CLICK_AND_OBSERVE(post-click)');
        const domDiagnostics = await page.evaluate(() => {
          const errs = [];
          document.querySelectorAll('[role="alert"], [aria-live="assertive"], [class*="toast"], [class*="alert"], [class*="error"]').forEach((el) => {
            const txt = (el.textContent || '').trim();
            if (txt && /error|warning|desync|expired|lost|reset|disabled|stale|failed|invalid/i.test(txt)) {
              errs.push('UI_DIAGNOSTIC_ALERT: ' + txt);
            }
          });
          document.querySelectorAll('[id*="counter"], [class*="counter"]').forEach((el) => {
            const val = parseInt((el.textContent || '').trim(), 10);
            if (!Number.isNaN(val) && val < 0) {
              errs.push('UI_STATE_INVARIANT: counter underflowed to ' + val + ' (expected >= 0)');
            }
          });
          return errs;
        }).catch(() => []);
        for (const d of domDiagnostics) {
          pageErrors.push(d);
        }
        let resultText = '';
        if (cmd.output_selector && String(cmd.output_selector).trim().length > 0) {
          const outEl = page.locator(cmd.output_selector);
          const outCount = await withTimeout(outEl.count(), 8000, 'Output selector count');
          if (outCount > 0) {
            resultText = ((await withTimeout(outEl.textContent({ timeout: 8000 }), 9000, 'Output textContent')) || '').trim();
          }
        } else {
          // Generic observation fallback: capture body text snippet or active heading/status
          const bodyText = (await withTimeout(page.evaluate(() => document.body ? document.body.innerText.slice(0, 500) : ''), 5000, 'body snippet')) || '';
          resultText = bodyText.trim();
        }
        sendLine({
          kind: 'CLICK_RESULT',
          clicked: true,
          page_errors: pageErrors.slice(),
          result_text: resultText,
        }, reqId);
      } else if (cmd.cmd === 'DRAG_AND_OBSERVE') {
        ensureActivePage('DRAG_AND_OBSERVE');
        pageErrors = [];
        const fromX = Number(cmd.from_x) || 0;
        const fromY = Number(cmd.from_y) || 0;
        const toX = Number(cmd.to_x) || 0;
        const toY = Number(cmd.to_y) || 0;
        const steps = Math.max(1, Math.min(100, Number(cmd.steps) || 10));

        let targetLocator = null;
        if (cmd.target_selector && String(cmd.target_selector).trim().length > 0) {
          targetLocator = page.locator(cmd.target_selector).first();
        } else if (cmd.role && cmd.accessible_name) {
          targetLocator = page.getByRole(cmd.role, { name: cmd.accessible_name, exact: true });
        } else if (cmd.selector && String(cmd.selector).trim().length > 0 && !String(cmd.selector).startsWith('page.')) {
          targetLocator = page.locator(cmd.selector).first();
        } else if (cmd.test_id) {
          targetLocator = page.getByTestId(cmd.test_id);
        } else {
          targetLocator = page.locator('button, [role="slider"]').first();
        }

        let initialBox = null;
        if (targetLocator && (await targetLocator.count()) > 0) {
          initialBox = await targetLocator.boundingBox();
        }

        const positions = [];
        await page.mouse.move(fromX, fromY);
        await page.mouse.down();
        for (let s = 1; s <= steps; s++) {
          const curX = Math.round(fromX + (toX - fromX) * (s / steps));
          const curY = Math.round(fromY + (toY - fromY) * (s / steps));
          await page.mouse.move(curX, curY);
          await page.waitForTimeout(15);
          if (targetLocator && (await targetLocator.count()) > 0) {
            const b = await targetLocator.boundingBox();
            if (b) {
              positions.push({ x: b.x, y: b.y, width: b.width, height: b.height });
            }
          }
        }
        await page.mouse.up();
        await page.waitForTimeout(50);
        ensureActivePage('DRAG_AND_OBSERVE(post-drag)');

        let finalBox = null;
        if (targetLocator && (await targetLocator.count()) > 0) {
          finalBox = await targetLocator.boundingBox();
        }

        let jitterScoreBps = 0;
        if (positions.length >= 3) {
          let accelDiffSum = 0;
          for (let i = 2; i < positions.length; i++) {
            const prevDx = positions[i - 1].x - positions[i - 2].x;
            const curDx = positions[i].x - positions[i - 1].x;
            const prevDy = positions[i - 1].y - positions[i - 2].y;
            const curDy = positions[i].y - positions[i - 1].y;
            const accelX = Math.abs(curDx - prevDx);
            const accelY = Math.abs(curDy - prevDy);
            if (accelX > 2 || accelY > 2) {
              accelDiffSum += Math.round((accelX + accelY) * 200);
            }
            if ((prevDx > 0 && curDx < -2) || (prevDx < 0 && curDx > 2)) {
              accelDiffSum += 2000;
            }
          }
          jitterScoreBps = Math.min(100000, Math.round(accelDiffSum / (positions.length - 2)));
        }

        const deltaX = finalBox && initialBox ? Math.round(finalBox.width - initialBox.width) : (toX - fromX);
        const deltaY = finalBox && initialBox ? Math.round(finalBox.height - initialBox.height) : (toY - fromY);

        sendLine({
          kind: 'DRAG_RESULT',
          dragged: true,
          page_errors: pageErrors.slice(),
          delta_x: deltaX,
          delta_y: deltaY,
          jitter_score_bps: jitterScoreBps,
          final_rect: finalBox ? {
            x: Math.round(finalBox.x),
            y: Math.round(finalBox.y),
            width: Math.max(0, Math.round(finalBox.width)),
            height: Math.max(0, Math.round(finalBox.height)),
          } : null,
        }, reqId);
      } else if (cmd.cmd === 'TYPE_AND_OBSERVE') {
        ensureActivePage('TYPE_AND_OBSERVE');
        pageErrors = [];
        let locator = null;
        if (cmd.role && cmd.accessible_name) {
          locator = page.getByRole(cmd.role, { name: cmd.accessible_name, exact: true });
        } else if (cmd.test_id) {
          locator = page.getByTestId(cmd.test_id);
        } else if (cmd.selector && String(cmd.selector).trim().length > 0) {
          locator = page.locator(cmd.selector).first();
        } else {
          locator = page.locator(':focus');
        }
        await locator.fill(String(cmd.text || ''));
        await page.waitForTimeout(50);
        ensureActivePage('TYPE_AND_OBSERVE(post-type)');
        sendLine({
          kind: 'TYPE_RESULT',
          typed: true,
          page_errors: pageErrors.slice(),
          result_text: String(cmd.text || ''),
        }, reqId);
      } else if (cmd.cmd === 'SET_NETWORK_PERTURBATION') {
        networkLatencyMs = typeof cmd.latency_ms === 'number' ? Math.max(0, cmd.latency_ms) : 0;
        networkJitterMs = typeof cmd.jitter_ms === 'number' ? Math.max(0, cmd.jitter_ms) : 0;
        sendLine({
          kind: 'PERTURBATION_SET',
          latency_ms: networkLatencyMs,
          jitter_ms: networkJitterMs,
        }, reqId);
      } else if (cmd.cmd === 'SHUTDOWN') {
        if (browser) {
          await withTimeout(browser.close(), 2000, 'browser.close').catch(() => {});
          browser = null;
          browserConnected = false;
          page = null;
        }
        sendLine({ kind: 'SHUTDOWN_OK' }, reqId);
        break;
      } else {
        sendLine({ kind: 'ERROR', message: 'Unknown command: ' + String(cmd.cmd) }, reqId);
      }
    } catch (err) {
      sendLine({
        kind: 'ERROR',
        message: String(err && err.message ? err.message : err),
      }, reqId);
    }
  }

  if (browser) {
    await withTimeout(browser.close(), 1500, 'browser.close').catch(() => {});
  }
  process.exit(0);
})();
"#;

/// Commands sent from Rust to the Node.js Playwright bridge over stdin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BridgeCommand {
    StartBrowser {
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
    },
    ProbePage {
        url: String,
        click_x: i32,
        click_y: i32,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        reload_page: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shadow_script_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shadow_script_body: Option<String>,
    },
    ClickAndObserve {
        #[serde(default)]
        role: String,
        #[serde(default)]
        accessible_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        test_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        click_x: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        click_y: Option<i32>,
        #[serde(default)]
        output_selector: String,
    },
    DragAndObserve {
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        #[serde(default = "default_drag_steps")]
        steps: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accessible_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_selector: Option<String>,
    },
    TypeAndObserve {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        accessible_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selector: Option<String>,
    },
    SetNetworkPerturbation {
        latency_ms: u64,
        jitter_ms: u64,
    },
    Shutdown,
}

const fn default_drag_steps() -> u32 {
    10
}

/// Responses sent from the Node.js Playwright bridge to Rust over stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BridgeResponse {
    Ready,
    ProbeResult {
        navigated: bool,
        aria_snapshot: String,
        candidates: Vec<DomCandidate>,
        page_errors: Vec<String>,
    },
    ClickResult {
        clicked: bool,
        page_errors: Vec<String>,
        result_text: String,
    },
    DragResult {
        dragged: bool,
        page_errors: Vec<String>,
        delta_x: i32,
        delta_y: i32,
        #[serde(default)]
        jitter_score_bps: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        final_rect: Option<crate::locator::BoundingBox>,
    },
    TypeResult {
        typed: bool,
        page_errors: Vec<String>,
        result_text: String,
    },
    PerturbationSet {
        latency_ms: u64,
        jitter_ms: u64,
    },
    ShutdownOk,
    Error {
        message: String,
    },
}

/// Embedded localhost HTTP server serving files from a `ScratchSandbox`.
#[derive(Debug)]
pub struct SandboxServer {
    port: u16,
    shutdown_flag: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl SandboxServer {
    /// Starts an HTTP server on `127.0.0.1:0` (or `preferred_port` if available, falling back to `:0`)
    /// serving static files from `root_dir`.
    pub fn start(root_dir: &Path, preferred_port: Option<u16>) -> Result<Self> {
        let bind_addr = preferred_port
            .map(|p| format!("127.0.0.1:{p}"))
            .unwrap_or_else(|| "127.0.0.1:0".to_string());

        let listener = match TcpListener::bind(&bind_addr) {
            Ok(l) => l,
            Err(_) if preferred_port.is_some() => TcpListener::bind("127.0.0.1:0")?,
            Err(e) => return Err(CoreError::Io(e)),
        };

        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;

        let shutdown_flag = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&shutdown_flag);
        let root = root_dir.to_path_buf();

        let worker = thread::spawn(move || {
            let mut conn_handles: Vec<JoinHandle<()>> = Vec::new();
            while !flag_clone.load(Ordering::Relaxed) {
                conn_handles.retain(|h| !h.is_finished());
                match listener.accept() {
                    Ok((stream, _)) => {
                        let root_clone = root.clone();
                        conn_handles.push(thread::spawn(move || {
                            let _ = handle_http_connection(stream, &root_clone);
                        }));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
            for handle in conn_handles {
                let _ = handle.join();
            }
        });

        Ok(Self {
            port,
            shutdown_flag,
            worker: Some(worker),
        })
    }

    /// The bound localhost TCP port (`127.0.0.1:<port>`).
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Base URL (`http://127.0.0.1:<port>`).
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Signals the server thread to stop and waits for it to join.
    pub fn shutdown(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for SandboxServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn handle_http_connection(mut stream: TcpStream, root: &Path) -> Result<()> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));

    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let Some(first_line) = req.lines().next() else {
        return Ok(());
    };
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let raw_target = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        return write_http_response(&mut stream, "405 Method Not Allowed", "text/plain", b"");
    }

    let path_part = raw_target.split('?').next().unwrap_or("/");
    let rel_path = match path_part.trim_start_matches('/') {
        "" => "index.html",
        other => other,
    };

    if rel_path == "favicon.ico" {
        return write_http_response(&mut stream, "204 No Content", "image/x-icon", b"");
    }

    let Ok(normalized) = TaskManifest::assert_safe_path(rel_path, "http_request_path") else {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    };

    if normalized
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    }

    let full_path = root.join(&normalized);
    let Ok(meta) = fs::symlink_metadata(&full_path) else {
        return write_http_response(&mut stream, "404 Not Found", "text/plain", b"Not Found");
    };
    if meta.file_type().is_symlink() || !meta.is_file() || meta.len() > MAX_HTTP_FILE_BYTES {
        return write_http_response(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
    }

    let body = fs::read(&full_path)?;
    let content_type = content_type_for(&normalized.to_string_lossy());
    write_http_response(&mut stream, "200 OK", content_type, &body)
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".html") || path.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn write_http_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

/// Reads a single `\n`-delimited line from `reader` with a hard byte limit (`max_bytes`)
/// so a broken child process streaming gigabytes without a newline cannot cause OOM.
fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            if buf.is_empty() {
                return Ok(None);
            }
            return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
        }
        if let Some(newline_pos) = available.iter().position(|&b| b == b'\n') {
            let take = newline_pos + 1;
            if buf.len().saturating_add(take) > max_bytes {
                reader.consume(take);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Bridge stdout line exceeded maximum size of {max_bytes} bytes"),
                ));
            }
            buf.extend_from_slice(&available[..take]);
            reader.consume(take);
            return Ok(Some(String::from_utf8_lossy(&buf).into_owned()));
        }
        let chunk_len = available.len();
        if buf.len().saturating_add(chunk_len) > max_bytes {
            reader.consume(chunk_len);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Bridge stdout line exceeded maximum size of {max_bytes} bytes"),
            ));
        }
        buf.extend_from_slice(available);
        reader.consume(chunk_len);
    }
}

fn spawn_stdout_reader(stdout: ChildStdout) -> Receiver<std::io::Result<String>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader, MAX_STDOUT_LINE_BYTES) {
                Ok(Some(line)) => {
                    if tx.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    break;
                }
            }
        }
    });
    rx
}

fn spawn_stderr_drainer(stderr: ChildStderr) -> Arc<Mutex<Vec<String>>> {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let buf_clone = Arc::clone(&buffer);
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        while let Ok(Some(line)) = read_bounded_line(&mut reader, 8192) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let capped: String = trimmed.chars().take(512).collect();
            if let Ok(mut guard) = buf_clone.lock() {
                if guard.len() >= MAX_STDERR_LINES {
                    guard.remove(0);
                }
                guard.push(capped);
            }
        }
    });
    buffer
}

/// Line-delimited JSON-lines stdio client managing the Node.js Playwright bridge child process.
#[derive(Debug)]
pub struct PlaywrightBridge {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout_rx: Receiver<std::io::Result<String>>,
    stderr_buf: Arc<Mutex<Vec<String>>>,
    script_path: PathBuf,
    next_request_id: u64,
    command_timeout: Duration,
    poisoned: bool,
    terminated: bool,
    shut_down: bool,
}

impl PlaywrightBridge {
    /// Materializes the bridge script at `sandbox_dir/.replayfix-playwright-bridge.cjs`,
    /// spawns `node`, and launches Chromium (headless or headed).
    pub fn start(
        sandbox_dir: &Path,
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Result<Self> {
        Self::start_with_script_and_timeouts(
            sandbox_dir,
            PLAYWRIGHT_BRIDGE_JS,
            headed,
            viewport_width,
            viewport_height,
            DEFAULT_BRIDGE_STARTUP_TIMEOUT,
            DEFAULT_BRIDGE_COMMAND_TIMEOUT,
        )
    }

    /// Starts the bridge using a custom script body and explicit startup/command timeouts.
    pub fn start_with_script_and_timeouts(
        sandbox_dir: &Path,
        script_source: &str,
        headed: bool,
        viewport_width: u32,
        viewport_height: u32,
        startup_timeout: Duration,
        command_timeout: Duration,
    ) -> Result<Self> {
        let script_path = sandbox_dir.join(BRIDGE_SCRIPT_FILENAME);
        fs::write(&script_path, script_source)?;

        let mut cmd = Platform::create_command("node");
        cmd.arg(&script_path)
            .current_dir(sandbox_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut search_roots = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            search_roots.push(cwd);
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                search_roots.push(parent.to_path_buf());
            }
        }
        'search: for root in search_roots {
            let mut cur: Option<&Path> = Some(root.as_path());
            while let Some(dir) = cur {
                let node_modules = dir.join("node_modules");
                if node_modules.exists() {
                    if let Some(nm_str) = node_modules.to_str() {
                        cmd.env("NODE_PATH", nm_str);
                    }
                    let pw_dir = node_modules.join("playwright");
                    if pw_dir.exists() {
                        if let Some(pw_str) = pw_dir.to_str() {
                            cmd.env("REPLAYFIX_PLAYWRIGHT_PATH", pw_str);
                        }
                        break 'search;
                    }
                }
                cur = dir.parent();
            }
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| CoreError::Browser(format!("Failed to spawn Node.js bridge: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stdin was not piped".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| CoreError::Browser("Bridge child stderr was not piped".into()))?;

        let stdout_rx = spawn_stdout_reader(stdout);
        let stderr_buf = spawn_stderr_drainer(stderr);

        let mut bridge = Self {
            child,
            stdin: Some(stdin),
            stdout_rx,
            stderr_buf,
            script_path,
            next_request_id: 1,
            command_timeout,
            poisoned: false,
            terminated: false,
            shut_down: false,
        };

        match bridge.send_command_with_timeout(
            &BridgeCommand::StartBrowser {
                headed,
                viewport_width,
                viewport_height,
            },
            startup_timeout,
        ) {
            Ok(BridgeResponse::Ready) => Ok(bridge),
            Ok(BridgeResponse::Error { message }) => {
                bridge.shutdown();
                Err(CoreError::Browser(message))
            }
            Ok(other) => {
                bridge.shutdown();
                Err(CoreError::Browser(format!(
                    "Unexpected response to START_BROWSER: {other:?}"
                )))
            }
            Err(e) => {
                bridge.shutdown();
                Err(e)
            }
        }
    }

    /// Child process ID for sandbox process tree supervision.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Returns captured stderr tail lines joined for diagnostic reporting.
    pub fn stderr_summary(&self) -> String {
        let Ok(guard) = self.stderr_buf.lock() else {
            return String::new();
        };
        guard.join(" | ")
    }

    /// Sends a single JSON-line command using the bridge's default command timeout.
    pub fn send_command(&mut self, command: &BridgeCommand) -> Result<BridgeResponse> {
        let timeout = self.command_timeout;
        self.send_command_with_timeout(command, timeout)
    }

    /// Sends a single JSON-line command with a monotonic `request_id` and awaits the matching
    /// response up to `timeout`. On timeout or protocol desynchronization, poisons the bridge
    /// and terminates the child process tree immediately.
    pub fn send_command_with_timeout(
        &mut self,
        command: &BridgeCommand,
        timeout: Duration,
    ) -> Result<BridgeResponse> {
        if self.shut_down || self.poisoned {
            return Err(CoreError::Browser(
                "Playwright bridge is already shut down or poisoned by a prior error".into(),
            ));
        }

        let req_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);

        let mut payload = serde_json::to_value(command)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("request_id".to_string(), serde_json::Value::from(req_id));
        }
        let mut line = serde_json::to_string(&payload)?;
        line.push('\n');

        let Some(stdin) = self.stdin.as_mut() else {
            self.poisoned = true;
            return Err(CoreError::Browser("Bridge stdin is closed".into()));
        };

        if let Err(e) = stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
        {
            self.poison_and_terminate();
            return Err(CoreError::Browser(format!(
                "Failed writing to bridge stdin (request_id={req_id}): {e}{}",
                self.format_stderr_suffix()
            )));
        }

        let deadline = Instant::now() + timeout;
        loop {
            let now = Instant::now();
            let Some(remaining) = deadline.checked_duration_since(now) else {
                self.poison_and_terminate();
                return Err(CoreError::CdpTimeout(format!(
                    "Playwright bridge timed out after {}ms waiting for request_id={req_id}{}",
                    timeout.as_millis(),
                    self.format_stderr_suffix()
                )));
            };

            match self.stdout_rx.recv_timeout(remaining) {
                Ok(Ok(raw_line)) => {
                    let trimmed = raw_line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let value: serde_json::Value = match serde_json::from_str(trimmed) {
                        Ok(v) => v,
                        Err(e) => {
                            self.poison_and_terminate();
                            return Err(CoreError::Browser(format!(
                                "Invalid JSON from Playwright bridge ('{}'): {e}{}",
                                truncate_str(trimmed, 240),
                                self.format_stderr_suffix()
                            )));
                        }
                    };

                    if let Some(resp_id) = value.get("request_id").and_then(|v| v.as_u64()) {
                        if resp_id < req_id {
                            // Late response from an earlier request; ignore and keep waiting for req_id.
                            continue;
                        }
                        if resp_id > req_id {
                            self.poison_and_terminate();
                            return Err(CoreError::Browser(format!(
                                "Protocol correlation mismatch: expected request_id={req_id}, received request_id={resp_id}"
                            )));
                        }
                    }

                    let parsed: BridgeResponse = serde_json::from_value(value).map_err(|e| {
                        self.poison_and_terminate();
                        CoreError::Browser(format!(
                            "Malformed BridgeResponse payload ('{}'): {e}",
                            truncate_str(trimmed, 240)
                        ))
                    })?;
                    return Ok(parsed);
                }
                Ok(Err(io_err)) => {
                    self.poison_and_terminate();
                    return Err(CoreError::Browser(format!(
                        "Failed reading from bridge stdout (request_id={req_id}): {io_err}{}",
                        self.format_stderr_suffix()
                    )));
                }
                Err(RecvTimeoutError::Timeout) => {
                    self.poison_and_terminate();
                    return Err(CoreError::CdpTimeout(format!(
                        "Playwright bridge timed out after {}ms waiting for request_id={req_id}{}",
                        timeout.as_millis(),
                        self.format_stderr_suffix()
                    )));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.poison_and_terminate();
                    return Err(CoreError::Browser(format!(
                        "Playwright bridge closed stdout unexpectedly (request_id={req_id}){}",
                        self.format_stderr_suffix()
                    )));
                }
            }
        }
    }

    fn format_stderr_suffix(&self) -> String {
        // Give the stderr drain thread a brief moment to flush any final crash lines.
        thread::sleep(Duration::from_millis(15));
        let summary = self.stderr_summary();
        if summary.is_empty() {
            String::new()
        } else {
            format!(" [stderr: {summary}]")
        }
    }

    fn poison_and_terminate(&mut self) {
        self.poisoned = true;
        let _ = self.stdin.take();
        if !self.terminated {
            self.terminated = true;
            let _ = Platform::terminate_process_tree(self.child.id());
            self.reap_child_bounded();
        }
    }

    fn reap_child_bounded(&mut self) {
        let deadline = Instant::now() + CHILD_REAP_TIMEOUT;
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => {
                    if Instant::now() >= deadline {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
    }

    /// Gracefully shuts down the browser, terminates the child process tree, reaps the
    /// child handle within a bounded deadline, and removes the temporary bridge script.
    ///
    /// Strictly idempotent: safe to call explicitly and again via `Drop`.
    pub fn shutdown(&mut self) {
        if self.shut_down {
            return;
        }
        if !self.poisoned {
            let _ =
                self.send_command_with_timeout(&BridgeCommand::Shutdown, SHUTDOWN_COMMAND_TIMEOUT);
        }
        self.shut_down = true;
        self.poison_and_terminate();
        let _ = fs::remove_file(&self.script_path);
    }
}

impl Drop for PlaywrightBridge {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let mut out: String = s.chars().take(max_chars).collect();
    if s.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::Fixture;
    use crate::locator::LocatorResolver;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    #[test]
    fn test_bridge_command_and_response_json_lines_roundtrip() {
        let cmd = BridgeCommand::ProbePage {
            url: "http://127.0.0.1:41234/index.html".into(),
            click_x: 160,
            click_y: 120,
            reload_page: false,
            shadow_script_path: None,
            shadow_script_body: None,
        };
        let encoded = serde_json::to_string(&cmd).unwrap();
        assert!(!encoded.contains('\n'));
        let decoded: BridgeCommand = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, cmd);
    }

    #[test]
    fn test_sandbox_server_serves_files_and_blocks_traversal() {
        let tmp = std::env::temp_dir().join(format!("replayfix-http-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join("index.html"), "<h1>Hello Sandbox</h1>").unwrap();
        fs::write(tmp.join("src/app.js"), "export const x = 1;").unwrap();

        let mut server = SandboxServer::start(&tmp, None).unwrap();
        assert!(server.port() > 0);

        // Open an idle connection that sends nothing to verify it cannot block subsequent requests or shutdown.
        let _idle_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        assert!(resp.contains("<h1>Hello Sandbox</h1>"));

        let mut bad_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        bad_stream
            .write_all(b"GET /../secret.txt HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut bad_resp = String::new();
        bad_stream.read_to_string(&mut bad_resp).unwrap();
        assert!(bad_resp.starts_with("HTTP/1.1 403 Forbidden"));

        let mut drive_stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
        drive_stream
            .write_all(b"GET /C:/Windows/win.ini HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut drive_resp = String::new();
        drive_stream.read_to_string(&mut drive_resp).unwrap();
        assert!(drive_resp.starts_with("HTTP/1.1 403 Forbidden"));

        server.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_command_timeout_terminates_hung_process_and_collects_stderr() {
        let tmp =
            std::env::temp_dir().join(format!("replayfix-timeout-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Mock bridge script that replies READY to START_BROWSER, then logs to stderr and hangs forever on PROBE_PAGE.
        let hung_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stderr.write('SIMULATED_CDP_STALL: execution context wedged\n');
    // Intentionally never write to stdout
    setInterval(() => {}, 1000);
  }
});
"#;

        let mut bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            hung_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_millis(300),
        )
        .unwrap();

        let start = Instant::now();
        let err = bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/index.html".into(),
                click_x: 100,
                click_y: 100,
                reload_page: false,
                shadow_script_path: None,
                shadow_script_body: None,
            })
            .unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "Timeout enforcement took too long: {elapsed:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("timed out"),
            "Expected timeout error, got: {msg}"
        );
        assert!(
            msg.contains("SIMULATED_CDP_STALL"),
            "Expected captured stderr in error, got: {msg}"
        );

        // Subsequent commands on a poisoned bridge must fail immediately without blocking.
        assert!(bridge.send_command(&BridgeCommand::Shutdown).is_err());

        // Shutdown must complete rapidly without hanging on the already-terminated child.
        bridge.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_correlation_skips_stale_id_and_rejects_future_id_and_handles_stderr_flood() {
        let tmp = std::env::temp_dir().join(format!("replayfix-corr-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // Script floods >128 KiB to stderr (testing pipe deadlock immunity), emits a stale request_id=0
        // before sending the matching request_id for PROBE_PAGE, and then sends a mismatched future request_id=999 on CLICK_AND_OBSERVE.
        let corr_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    for (let i = 0; i < 1500; i++) {
      process.stderr.write('STDERR_FLOOD_LINE_' + i + '_' + 'X'.repeat(80) + '\n');
    }
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    // Emit a late response from an earlier request_id (0 < cmd.request_id)
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: 0 }) + '\n');
    // Then emit the correlated response
    process.stdout.write(JSON.stringify({
      kind: 'PROBE_RESULT',
      request_id: cmd.request_id,
      navigated: true,
      aria_snapshot: '- button "OK"',
      candidates: [],
      page_errors: []
    }) + '\n');
  } else if (cmd.cmd === 'CLICK_AND_OBSERVE') {
    // Emit a corrupted future request_id=999
    process.stdout.write(JSON.stringify({
      kind: 'CLICK_RESULT',
      request_id: 999,
      clicked: true,
      page_errors: [],
      result_text: 'wrong'
    }) + '\n');
  }
});
"#;

        let mut bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            corr_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();

        // PROBE_PAGE should skip stale request_id=0 and return the matching PROBE_RESULT.
        let probe = bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
                reload_page: false,
                shadow_script_path: None,
                shadow_script_body: None,
            })
            .unwrap();
        assert!(matches!(
            probe,
            BridgeResponse::ProbeResult {
                navigated: true,
                ..
            }
        ));

        // CLICK_AND_OBSERVE receives mismatched request_id=999 and must fail closed.
        let err = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: "button".into(),
                accessible_name: "OK".into(),
                test_id: None,
                selector: None,
                click_x: None,
                click_y: None,
                output_selector: "#out".into(),
            })
            .unwrap_err();
        assert!(err.to_string().contains("Protocol correlation mismatch"));

        bridge.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_bridge_unexpected_eof_and_malformed_json_and_unresponsive_shutdown() {
        let tmp = std::env::temp_dir().join(format!("replayfix-eof-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        // 1. Child exits abruptly (EOF) during PROBE_PAGE after writing to stderr.
        let eof_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stderr.write('FATAL_BRIDGE_CRASH: simulated segfault\n');
    process.exit(1);
  }
});
"#;
        let mut eof_bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            eof_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();
        let eof_err = eof_bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
                reload_page: false,
                shadow_script_path: None,
                shadow_script_body: None,
            })
            .unwrap_err();
        assert!(eof_err.to_string().contains("closed stdout unexpectedly"));
        assert!(eof_err.to_string().contains("FATAL_BRIDGE_CRASH"));
        eof_bridge.shutdown();

        // 2. Child emits malformed JSON and ignores SHUTDOWN command.
        let malformed_script = r#"
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
rl.on('line', (line) => {
  const cmd = JSON.parse(line);
  if (cmd.cmd === 'START_BROWSER') {
    process.stdout.write(JSON.stringify({ kind: 'READY', request_id: cmd.request_id }) + '\n');
  } else if (cmd.cmd === 'PROBE_PAGE') {
    process.stdout.write('{not-valid-json\n');
  } else if (cmd.cmd === 'SHUTDOWN') {
    // Intentionally ignore SHUTDOWN to verify forced termination
    setInterval(() => {}, 1000);
  }
});
"#;
        let mut bad_bridge = PlaywrightBridge::start_with_script_and_timeouts(
            &tmp,
            malformed_script,
            false,
            1280,
            720,
            Duration::from_secs(5),
            Duration::from_secs(5),
        )
        .unwrap();
        let json_err = bad_bridge
            .send_command(&BridgeCommand::ProbePage {
                url: "http://127.0.0.1:9999/".into(),
                click_x: 10,
                click_y: 10,
                reload_page: false,
                shadow_script_path: None,
                shadow_script_body: None,
            })
            .unwrap_err();
        assert!(json_err.to_string().contains("Invalid JSON"));
        bad_bridge.shutdown();

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_live_playwright_bridge_reverse_probes_and_clicks_rf01() {
        let tmp = std::env::temp_dir().join(format!("replayfix-pw-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();

        let fixture = Fixture::for_task("RF-01").unwrap();
        fs::write(tmp.join("index.html"), fixture.index_html).unwrap();
        fs::write(tmp.join(fixture.target_path), fixture.buggy_source).unwrap();

        let mut server = SandboxServer::start(&tmp, None).unwrap();
        let mut bridge =
            PlaywrightBridge::start(&tmp, false, fixture.viewport_width, fixture.viewport_height)
                .unwrap();

        let url = format!("{}/index.html", server.base_url());
        let probe = bridge
            .send_command(&BridgeCommand::ProbePage {
                url,
                click_x: fixture.click_x,
                click_y: fixture.click_y,
                reload_page: false,
                shadow_script_path: None,
                shadow_script_body: None,
            })
            .unwrap();

        let BridgeResponse::ProbeResult {
            navigated,
            aria_snapshot,
            candidates,
            page_errors,
        } = probe
        else {
            panic!("Expected ProbeResult, got {probe:?}");
        };
        assert!(navigated);
        assert!(page_errors.is_empty());

        let grounded = LocatorResolver::resolve(
            fixture.click_x,
            fixture.click_y,
            fixture.viewport_width,
            fixture.viewport_height,
            &aria_snapshot,
            &candidates,
        )
        .unwrap();
        assert_eq!(
            grounded.selector,
            "page.getByRole('button', { name: 'Refresh Cart', exact: true })"
        );

        let click_res = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: grounded.candidate.role,
                accessible_name: grounded.candidate.accessible_name,
                test_id: grounded.candidate.test_id,
                selector: None,
                click_x: None,
                click_y: None,
                output_selector: "#cart-summary-output".into(),
            })
            .unwrap();

        let BridgeResponse::ClickResult {
            clicked,
            page_errors,
            result_text,
        } = click_res
        else {
            panic!("Expected ClickResult, got {click_res:?}");
        };
        assert!(clicked);
        assert!(
            !page_errors.is_empty(),
            "Expected Gate 1 TypeError in page_errors"
        );
        assert_eq!(result_text, "Pending");

        // Verify TOCTOU detachment detection when clicking a non-existent/detached element.
        let detached_res = bridge
            .send_command(&BridgeCommand::ClickAndObserve {
                role: "button".into(),
                accessible_name: "Detached Button".into(),
                test_id: None,
                selector: None,
                click_x: None,
                click_y: None,
                output_selector: "#cart-summary-output".into(),
            })
            .unwrap();
        match detached_res {
            BridgeResponse::Error { message } => {
                assert!(
                    message.contains("DOM_NODE_DETACHED"),
                    "Expected DOM_NODE_DETACHED error, got: {message}"
                );
            }
            other => panic!("Expected Error for detached node, got: {other:?}"),
        }

        bridge.shutdown();
        server.shutdown();
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_drag_and_type_bridge_commands_roundtrip() {
        let drag_cmd = BridgeCommand::DragAndObserve {
            from_x: 100,
            from_y: 100,
            to_x: 200,
            to_y: 200,
            steps: 5,
            role: None,
            accessible_name: None,
            selector: Some(".resize-handle".into()),
            target_selector: Some(".box".into()),
        };
        let serialized = serde_json::to_string(&drag_cmd).unwrap();
        assert!(serialized.contains("DRAG_AND_OBSERVE"));
        assert!(serialized.contains(".resize-handle"));

        let drag_res_json = serde_json::json!({
            "kind": "DRAG_RESULT",
            "request_id": 1,
            "dragged": true,
            "page_errors": [],
            "delta_x": 100,
            "delta_y": 100,
            "jitter_score_bps": 420,
            "final_rect": {
                "x": 100,
                "y": 100,
                "width": 300,
                "height": 300
            }
        });
        let drag_res: BridgeResponse = serde_json::from_value(drag_res_json).unwrap();
        match drag_res {
            BridgeResponse::DragResult {
                dragged,
                delta_x,
                jitter_score_bps,
                final_rect,
                ..
            } => {
                assert!(dragged);
                assert_eq!(delta_x, 100);
                assert_eq!(jitter_score_bps, 420);
                assert_eq!(final_rect.unwrap().width, 300);
            }
            other => panic!("Expected DragResult, got {other:?}"),
        }

        let type_cmd = BridgeCommand::TypeAndObserve {
            text: "Hello World".into(),
            role: Some("textbox".into()),
            accessible_name: Some("Search".into()),
            selector: None,
        };
        let type_ser = serde_json::to_string(&type_cmd).unwrap();
        assert!(type_ser.contains("TYPE_AND_OBSERVE"));

        let type_res_json = serde_json::json!({
            "kind": "TYPE_RESULT",
            "request_id": 2,
            "typed": true,
            "page_errors": [],
            "result_text": "Hello World"
        });
        let type_res: BridgeResponse = serde_json::from_value(type_res_json).unwrap();
        assert!(matches!(
            type_res,
            BridgeResponse::TypeResult { typed: true, .. }
        ));
    }
}
