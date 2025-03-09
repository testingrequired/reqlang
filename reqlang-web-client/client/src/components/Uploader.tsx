import { useState } from "react";
import "./Uploader.css";

type LoadedFile = {
  fileName: string;
  text: string;
};

const DragDropFileReader = () => {
  const [file, setFile] = useState<LoadedFile | null>(null);
  const [dragging, setDragging] = useState(false);

  const handleDrop = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);

    if (event.dataTransfer.files && event.dataTransfer.files.length > 0) {
      const file = event.dataTransfer.files[0];
      const reader = new FileReader();

      reader.onload = (e) => {
        setFile({
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
      {file ? (
        <>
          <h2>📄 {file.fileName}</h2>

          <pre className="reqfile-text">{file.text}</pre>
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
                  setFile({
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
