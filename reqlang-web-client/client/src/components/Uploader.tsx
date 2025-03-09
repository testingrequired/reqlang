import { useState } from "react";
import "./Uploader.css";

const DragDropFileReader = () => {
  const [text, setText] = useState<string | null>(null);
  const [fileName, setFilename] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);

  const handleDrop = (event: React.DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    setDragging(false);

    if (event.dataTransfer.files && event.dataTransfer.files.length > 0) {
      const file = event.dataTransfer.files[0];
      const reader = new FileReader();

      console.log(JSON.stringify(event.dataTransfer));

      reader.onload = (e) => {
        setText(e.target?.result as string);
      };

      console.log(JSON.stringify(event.dataTransfer.files, null, 2));

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
            setFilename(file.name);
          }
        }}
        className="hidden"
        data-testid="uploader"
      />
      {text && (
        <div>
          <h3>{fileName}</h3>
          <pre>{text}</pre>
        </div>
      )}
    </div>
  );
};

export default DragDropFileReader;
