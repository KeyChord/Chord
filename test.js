ObjC.import('ApplicationServices');

function getAttr(el, attr) {
  const ref = Ref();
  const err = $.AXUIElementCopyAttributeValue(el, $(attr), ref);
  if (err !== 0 || !ref[0]) return null;

  try {
    return ObjC.unwrap(ref[0]);
  } catch (e) {
    return null;
  }
}

function getAppByName(name) {
  const system = $.AXUIElementCreateSystemWide();

  const appsRef = Ref();
  const err = $.AXUIElementCopyAttributeValue(system, $('AXChildren'), appsRef);

  if (err !== 0 || !appsRef[0]) {
    console.log("failed to get apps");
    return null;
  }

  const apps = ObjC.unwrap(appsRef[0]);

  for (const app of apps) {
    const title = getAttr(app, 'AXTitle');
    if (title === name) {
      return app;
    }
  }

  return null;
}

const app = getAppByName('Control Center');

if (!app) {
  console.log("not found");
} else {
  console.log("found Control Center");
}
