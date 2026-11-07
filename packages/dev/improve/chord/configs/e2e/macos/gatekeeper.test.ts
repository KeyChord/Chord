import { expect, test } from "bun:test";
import { launchProblems } from "#gatekeeper";

/** `guestVerdicts` as macOS 27.0.1 logged 0.2.0's first launch, which it refused. */
const REFUSED_0_2_0 = `Timestamp               Ty Process[PID:TID]
2026-11-07 11:41:54.400 Df syspolicyd[530:1b26d27] [com.apple.securityd:gk] rule 11 applies - allow=1
2026-11-07 11:41:54.406 Df syspolicyd[530:1b252b3] [com.apple.securityd:gk] rule 11 applies - allow=1
2026-11-07 11:41:54.408 E  XprotectService[12243:1b26d23] [com.apple.xprotect:xprotect] File /Applications/Chord.app/Contents/MacOS/chord failed on rPathCmd /Users/runner/work/_temp/bun-embed/libbun_embed.dylib (rpath resolved to: (path not found), bundleURL: /Applications/Chord.app)
2026-11-07 11:41:55.266 Df CoreServicesUIAgent[655:1b26d09] [com.apple.launchservices:uiagent] -[GKQuarantineResolver malwareCheckEnded:result:forURL:]: XProtect finished executable analysis {
    XProtectCaspianContext =     {
        "context:uti" = "com.apple.application-bundle";
        operation = "operation:execute";
    };
    XProtectIsNotarized = 1;
    XProtectMalwareType = 4;
}
`;

/** The same launch as it should go, with the verdict a notarized download gets on the same Mac. */
const OPENED = `Timestamp               Ty Process[PID:TID]
2026-11-07 11:41:54.400 Df syspolicyd[530:1b26d27] [com.apple.securityd:gk] rule 11 applies - allow=1
2026-11-07 11:41:55.266 Df CoreServicesUIAgent[655:1b26d09] [com.apple.launchservices:uiagent] -[GKQuarantineResolver malwareCheckEnded:result:forURL:]: XProtect finished executable analysis {
    XProtectCaspianContext =     {
        "context:uti" = "com.apple.application-bundle";
        operation = "operation:execute";
    };
    XProtectIsNotarized = 1;
    XProtectMalwareType = 0;
}
`;

/** The disk image's verdict from that session, which isn't the app being opened. */
const DISK_IMAGE_ONLY = `Timestamp               Ty Process[PID:TID]
2026-11-07 11:41:25.752 Df syspolicyd[530:1b24567] [com.apple.securityd:gk] rule 16 applies - allow=1
2026-11-07 11:41:25.759 Df CoreServicesUIAgent[655:1b2455a] [com.apple.launchservices:uiagent] -[GKQuarantineResolver malwareCheckEnded:result:forURL:]: XProtect finished executable analysis {
    XProtectCaspianContext =     {
        LSDownloadContentTypeKey = "com.apple.disk-image-udif";
        "context:primary-signature" = 1;
        "context:uti" = "com.apple.disk-image-udif";
        operation = "operation:lsopen";
    };
    XProtectIsNotarized = 1;
    XProtectMalwareType = 0;
    XProtectShouldTranslocateDMG = 0;
}
`;

test("0.2.0's refusal is caught by the rpath XProtect rejected and by its verdict", () => {
  expect(launchProblems(REFUSED_0_2_0)).toEqual([
    "2026-11-07 11:41:54.408 E  XprotectService[12243:1b26d23] [com.apple.xprotect:xprotect] File /Applications/Chord.app/Contents/MacOS/chord failed on rPathCmd /Users/runner/work/_temp/bun-embed/libbun_embed.dylib (rpath resolved to: (path not found), bundleURL: /Applications/Chord.app)",
    "XProtect refused it (XProtectMalwareType = 4)",
  ]);
});

test("a notarized app that XProtect lets through has no problems", () => {
  expect(launchProblems(OPENED)).toEqual([]);
});

test("a launch XProtect never looked at doesn't pass as checked", () => {
  expect(launchProblems(DISK_IMAGE_ONLY)).toEqual(["XProtect never analyzed the app as it opened"]);
  expect(launchProblems("")).toEqual(["XProtect never analyzed the app as it opened"]);
});

test("an unnotarized app and a Gatekeeper refusal are problems", () => {
  const unnotarized = OPENED.replace("XProtectIsNotarized = 1;", "XProtectIsNotarized = 0;");
  expect(launchProblems(unnotarized)).toEqual(["XProtect didn't find it notarized"]);
  const denied = OPENED.replace("allow=1", "allow=0");
  expect(launchProblems(denied)).toEqual([
    "2026-11-07 11:41:54.400 Df syspolicyd[530:1b26d27] [com.apple.securityd:gk] rule 11 applies - allow=0",
  ]);
});
