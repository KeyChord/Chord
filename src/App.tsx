import { useEffect } from "react";
import { ChordIndicatorWindow } from "./routes/chords";
import { Toaster } from "./components/ui/sonner";
import { SettingsWindow } from "./windows/settings";

function App() {
  useEffect(() => {
    const rootElement = document.getElementById("root");
    const fullscreenClasses = ["m-0", "h-full", "w-full", "bg-transparent", "p-0"];

    [document.documentElement, document.body, rootElement].forEach((element) => {
      element?.classList.add(...fullscreenClasses);
    });

    return () => {
      [document.documentElement, document.body, rootElement].forEach((element) => {
        element?.classList.remove(...fullscreenClasses);
      });
    };
  }, []);

  const windowLabel = getCurrentWindow().label;

  if (windowLabel === "chords") {
    console.log('rendering chords window')
    return <ChordIndicatorWindow />;
  }

  return (
    <>
      <SettingsWindow />
      <Toaster position="top-right" />
    </>
  );
}

export default App;
