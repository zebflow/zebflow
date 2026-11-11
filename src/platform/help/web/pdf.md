# zeb/pdf

```tsx
import { createDocument, createTable, PAGE_SIZES } from "zeb/pdf";
```

`zeb/pdf` is the bundled client-side PDF generation library for Zebflow pages.

Use it when you need:

- downloadable reports
- invoices
- exportable tables
- project-native generated PDFs

Typical pattern:

1. build the document in a page
2. generate the PDF in browser
3. offer download or preview

See `help("web/libraries")` for the full API overview.

## Images

`.image({ data, x, y, width, height })` places a PNG or a JPEG.

- `data`: a base64 data URL, a `Uint8Array`, an `ArrayBuffer` or an array of
  bytes. The format is read from the bytes, not from the data URL's type.
- `width` / `height`: the size on the page, in points. Give one and the other
  keeps the image's proportions; give neither and the image takes one point
  per pixel.
- PNG transparency is kept: a signature on a transparent background shows the
  page through it.
- Refused with a message: webp and gif (convert them to PNG or JPEG), and
  interlaced PNG (save it without interlacing).
- The same image used twice is stored once.
