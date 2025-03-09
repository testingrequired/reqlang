import { useEffect } from "react";
import "./App.css";
import Uploader from "./components/Uploader";
import { useFileStore } from "./stores/file";
import { ParseResult } from "reqlang-types";

function App() {
  const fileStore = useFileStore();

  useEffect(() => {
    if (fileStore.file) {
      (async function () {
        const response = await fetch("/parse", {
          method: "POST",
          body: JSON.stringify({ payload: fileStore.file?.text }),
          headers: { "Content-Type": "application/json" },
        });

        if (response.ok) {
          const parsedRequestFile = (await response.json()) as ParseResult;

          fileStore.setParsedFile(parsedRequestFile);
        }
      })();
    }
  }, [fileStore.file]);

  return (
    <>
      <h1>reqlang-web-client</h1>

      <Uploader />

      {fileStore.parsedFile && (
        <>
          <h2>Parsed</h2>

          <pre className="parsed-file">
            {JSON.stringify(fileStore.parsedFile, null, 2)}
          </pre>
        </>
      )}
    </>
  );
}

export default App;
