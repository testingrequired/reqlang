import { useState } from "react";
import "./Uploader.css";

const DragDropFileReader = () => {
  const [text, setText] = useState<string | null>(null);
  const [filename, setFilename] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  const handleDrop = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);

    if (event.dataTransfer.files && event.dataTransfer.files.length > 0) {
      const file = event.dataTransfer.files[0];
      const reader = new FileReader();

      setFilename(file.name);

      reader.onload = (e) => {
        setText(e.target?.result as string);
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
      <p>Drag & drop a file here or click to select one</p>
      <input
        type="file"
        onChange={(e) => {
          if (e.target.files) {
            const file = e.target.files[0];
            const reader = new FileReader();
            reader.onload = (e) => {
              setText(e.target?.result as string);
            };
            reader.readAsText(file);
          }
        }}
        className="hidden"
        data-testid="uploader"
      />
      {text && (
        <div>
          <h2>{filename}</h2>
          <pre>{text}</pre>
        </div>
      )}
    </div>
  );
};

export default DragDropFileReader;
