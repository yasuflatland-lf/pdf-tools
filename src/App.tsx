import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

function App() {
  const [pdfiumStatus, setPdfiumStatus] = useState("Checking PDFium...");

  useEffect(() => {
    void invoke<string>("pdfium_health")
      .then((version) => setPdfiumStatus(version))
      .catch((error: unknown) => setPdfiumStatus(String(error)));
  }, []);

  return (
    <main className="flex min-h-screen items-center justify-center bg-slate-950 px-6 text-slate-100">
      <section className="max-w-xl text-center">
        <h1 className="text-5xl font-semibold tracking-tight">PDF Tools</h1>
        <p className="mt-4 text-lg text-slate-300">Combine PDFs and images into a single PDF.</p>
        <p className="mt-6 text-sm text-slate-400">PDFium: {pdfiumStatus}</p>
      </section>
    </main>
  );
}

export default App;
