import { useEffect, useState } from "react";
import "./Uploader.css";
import { useFileStore } from "../stores/file";

const DragDropFileReader = () => {
  const [dragging, setDragging] = useState(false);

  const fileStore = useFileStore();

  useEffect(() => {
    if (fileStore.file) {
      fetch("/parse", {
        method: "POST",
        body: JSON.stringify({ payload: fileStore.file?.text }),
        headers: { "Content-Type": "application/json" },
      });
    }
  }, [fileStore.file]);

  const handleDrop = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);

    if (event.dataTransfer.files && event.dataTransfer.files.length > 0) {
      const file = event.dataTransfer.files[0];
      const reader = new FileReader();

      reader.onload = (e) => {
        fileStore.setFile({
          fileName: file.name,
          text: e.target?.result as string,
        });
      };

      reader.readAsText(file);
    }
  };

  return (
    <div
      onDragOver={(e) => {
        e.preventDefault();
        setDragging(true);
      }}
      onDragLeave={() => setDragging(false)}
      onDrop={handleDrop}
      className={`border-2 border-dashed p-6 rounded-lg ${
        dragging ? "border-blue-500" : "border-gray-300"
      }`}
    >
      {fileStore.file ? (
        <>
          <h2>📄 {fileStore.file.fileName}</h2>

          <pre className="reqfile-text">{fileStore.file.text}</pre>
        </>
      ) : (
        <>
          <p>📄 Drag & drop a request file</p>

          <input
            type="file"
            onChange={(e) => {
              if (e.target.files) {
                const file = e.target.files[0];
                const reader = new FileReader();
                reader.onload = (e) => {
                  fileStore.setFile({
                    fileName: file.name,
                    text: e.target?.result as string,
                  });
                };
                reader.readAsText(file);
              }
            }}
            className="hidden"
            data-testid="uploader"
          />
        </>
      )}
    </div>
  );
};

export default DragDropFileReader;
