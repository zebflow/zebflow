import { useEffect, useRef, useState } from 'rwe';

interface PdfRendererProps {
  /** IR document node or DocumentBuilder instance */
  document: any;
  /** Width of the embedded iframe viewer. Default: '100%' */
  width?: string;
  /** Height of the embedded iframe viewer. Default: '800px' */
  height?: string;
  /** Called when PDF bytes are ready */
  onReady?: (bytes: Uint8Array) => void;
  /** Called on render error */
  onError?: (err: Error) => void;
  /** Show download button */
  showDownload?: boolean;
  /** Filename for download. Default: 'document.pdf' */
  filename?: string;
}

declare const __ZEB_PDF__: any;

export function PdfRenderer({
  document,
  width = '100%',
  height = '800px',
  onReady,
  onError,
  showDownload = false,
  filename = 'document.pdf',
}: PdfRendererProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [ms, setMs] = useState<number | null>(null);

  useEffect(() => {
    if (!document) return;

    let url: string | null = null;

    try {
      const { IrRenderer } = __ZEB_PDF__;
      const renderer = new IrRenderer();

      const doc = document._node ?? document;

      const t0 = performance.now();
      const bytes = renderer.renderDocument(doc);
      const elapsed = performance.now() - t0;

      const blob = new Blob([bytes], { type: 'application/pdf' });
      url = URL.createObjectURL(blob);

      if (iframeRef.current) {
        iframeRef.current.src = url;
      }

      setBlobUrl(url);
      setMs(Math.round(elapsed * 10) / 10);
      setError(null);
      onReady?.(bytes);
    } catch (e: any) {
      setError(e?.message ?? String(e));
      onError?.(e);
    }

    return () => {
      if (url) URL.revokeObjectURL(url);
    };
  }, [document]);

  function handleDownload() {
    if (!blobUrl) return;
    const a = window.document.createElement('a');
    a.href = blobUrl;
    a.download = filename;
    a.click();
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
      {(showDownload || ms !== null) && (
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px', fontSize: '13px', color: '#888' }}>
          {ms !== null && <span>rendered in {ms}ms</span>}
          {showDownload && blobUrl && (
            <button onClick={handleDownload} style={{ cursor: 'pointer' }}>
              Download {filename}
            </button>
          )}
        </div>
      )}
      {error ? (
        <div style={{ padding: '12px', background: '#fee', color: '#c00', borderRadius: '4px', fontFamily: 'monospace', fontSize: '13px' }}>
          {error}
        </div>
      ) : (
        <iframe
          ref={iframeRef}
          style={{ width, height, border: '1px solid #ddd', borderRadius: '4px' }}
        />
      )}
    </div>
  );
}
