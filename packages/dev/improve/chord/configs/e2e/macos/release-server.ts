/**
 * Lets a VM go through the real download flow with a build or a website that isn't published:
 * it serves a website to the VM with its download links pointing at a local DMG. Safari
 * downloads that file like any other, which is the whole point. It's reachable only from VMs,
 * on the host's address on their private network.
 */
import type { Guest } from "#guest";

export const LIVE_SITE = "https://trychord.app";
const DOWNLOAD_NAME = "Chord-aarch64.dmg";
const LOCAL_DOWNLOAD_PATH = `/__chord-e2e/${DOWNLOAD_NAME}`;

export interface ReleaseServer {
  url: string;
  stop(): void;
}

/** Rewrites what a site says about the published DMG so the VM downloads the local one. */
export function pointDownloadsAt(html: string, localPath: string): string {
  const published = /https:\/\/github\.com\/KeyChord\/Chord\/releases\/[^"'\s<>]*Chord-aarch64\.dmg/g;
  // Script bundles build the link at runtime, so also catch the click itself.
  const script = `<script>document.addEventListener("click",(e)=>{const a=e.target.closest?.('a[href*="${DOWNLOAD_NAME}"]');if(a)a.href=${JSON.stringify(localPath)}},true)</script>`;
  const rewritten = html.replace(published, localPath);
  return rewritten.includes("</head>")
    ? rewritten.replace("</head>", `${script}</head>`)
    : script + rewritten;
}

/** The host's address as seen from the guest. */
async function hostAddress(guest: Guest): Promise<string> {
  const { stdout } = await guest.run("route -n get default | awk '/gateway/ { print $2 }'");
  const address = stdout.trim();
  if (!address) throw new Error("cannot find the VM's default gateway");
  return address;
}

export async function startReleaseServer(
  guest: Guest,
  { site, dmg }: { site: string; dmg?: string },
): Promise<ReleaseServer> {
  const origin = new URL(site).origin;
  const server = Bun.serve({
    hostname: await hostAddress(guest),
    port: 0,
    async fetch(request) {
      const { pathname, search } = new URL(request.url);
      if (dmg && pathname === LOCAL_DOWNLOAD_PATH) {
        return new Response(Bun.file(dmg), {
          headers: {
            "content-type": "application/octet-stream",
            "content-disposition": `attachment; filename="${DOWNLOAD_NAME}"`,
          },
        });
      }
      const upstream = await fetch(origin + pathname + search, {
        method: request.method,
        headers: { "accept-encoding": "identity", accept: request.headers.get("accept") ?? "*/*" },
        redirect: "follow",
      });
      const headers = new Headers(upstream.headers);
      for (const name of ["content-encoding", "content-length", "content-security-policy"]) {
        headers.delete(name);
      }
      if (dmg && upstream.headers.get("content-type")?.includes("text/html")) {
        return new Response(pointDownloadsAt(await upstream.text(), LOCAL_DOWNLOAD_PATH), {
          status: upstream.status,
          headers,
        });
      }
      return new Response(upstream.body, { status: upstream.status, headers });
    },
  });
  return { url: server.url.origin, stop: () => void server.stop(true) };
}
