import { useState, useCallback } from "zeb";

// ── Student data ──────────────────────────────────────────────────────────────

const STUDENT = {
  name: "Ahmad Rizki Pratama",
  nim: "2021-CS-0451",
  program: "Bachelor of Computer Science",
  faculty: "Faculty of Engineering and Computer Science",
  entranceYear: 2021,
  status: "Active",
  totalCredits: 127,
  cumulativeGpa: 3.74,
};

const SEMESTERS = [
  {
    label: "Semester 1 — Academic Year 2021/2022",
    gpa: 3.82,
    credits: 15,
    courses: [
      { code: "CS101", name: "Introduction to Computer Science", cr: 3, grade: "A",  pts: 4.0 },
      { code: "MATH101", name: "Calculus I",                      cr: 3, grade: "A-", pts: 3.7 },
      { code: "PHYS101", name: "Physics I",                       cr: 3, grade: "B+", pts: 3.3 },
      { code: "ENG101",  name: "Technical English",               cr: 2, grade: "A",  pts: 4.0 },
      { code: "CS102",   name: "Introduction to Programming",     cr: 4, grade: "A",  pts: 4.0 },
    ],
  },
  {
    label: "Semester 2 — Academic Year 2021/2022",
    gpa: 3.71,
    credits: 17,
    courses: [
      { code: "CS201",  name: "Data Structures and Algorithms",  cr: 4, grade: "A",  pts: 4.0 },
      { code: "MATH201",name: "Calculus II",                     cr: 3, grade: "B+", pts: 3.3 },
      { code: "CS202",  name: "Object-Oriented Programming",     cr: 4, grade: "A-", pts: 3.7 },
      { code: "STAT101",name: "Probability and Statistics",      cr: 3, grade: "A",  pts: 4.0 },
      { code: "CS203",  name: "Computer Organization",           cr: 3, grade: "B+", pts: 3.3 },
    ],
  },
  {
    label: "Semester 3 — Academic Year 2022/2023",
    gpa: 3.79,
    credits: 20,
    courses: [
      { code: "CS301",  name: "Database Management Systems",     cr: 4, grade: "A",  pts: 4.0 },
      { code: "CS302",  name: "Operating Systems",               cr: 3, grade: "A-", pts: 3.7 },
      { code: "CS303",  name: "Computer Networks",               cr: 3, grade: "B+", pts: 3.3 },
      { code: "CS304",  name: "Software Engineering",            cr: 4, grade: "A",  pts: 4.0 },
      { code: "MATH301",name: "Linear Algebra",                  cr: 3, grade: "A-", pts: 3.7 },
      { code: "CS305",  name: "Web Development Fundamentals",    cr: 3, grade: "A",  pts: 4.0 },
    ],
  },
  {
    label: "Semester 4 — Academic Year 2022/2023",
    gpa: 3.83,
    credits: 17,
    courses: [
      { code: "CS401",  name: "Artificial Intelligence",          cr: 4, grade: "A",  pts: 4.0 },
      { code: "CS402",  name: "Machine Learning",                 cr: 3, grade: "A",  pts: 4.0 },
      { code: "CS403",  name: "Computer Vision",                  cr: 3, grade: "B+", pts: 3.3 },
      { code: "CS404",  name: "Algorithm Design & Analysis",      cr: 4, grade: "A-", pts: 3.7 },
      { code: "CS405",  name: "Mobile Application Development",   cr: 3, grade: "A",  pts: 4.0 },
    ],
  },
  {
    label: "Semester 5 — Academic Year 2023/2024",
    gpa: 3.63,
    credits: 20,
    courses: [
      { code: "CS501",  name: "Distributed Systems",              cr: 4, grade: "A-", pts: 3.7 },
      { code: "CS502",  name: "Cloud Computing",                  cr: 3, grade: "A",  pts: 4.0 },
      { code: "CS503",  name: "Cybersecurity Fundamentals",       cr: 3, grade: "B+", pts: 3.3 },
      { code: "CS504",  name: "Big Data Analytics",               cr: 4, grade: "A",  pts: 4.0 },
      { code: "CS505",  name: "Embedded Systems",                 cr: 3, grade: "B",  pts: 3.0 },
      { code: "CS506",  name: "Compiler Design",                  cr: 3, grade: "A-", pts: 3.7 },
    ],
  },
  {
    label: "Semester 6 — Academic Year 2023/2024",
    gpa: 3.84,
    credits: 15,
    courses: [
      { code: "CS601",  name: "Final Year Project I",              cr: 4, grade: "A",  pts: 4.0 },
      { code: "CS602",  name: "Blockchain Technology",             cr: 3, grade: "A-", pts: 3.7 },
      { code: "CS603",  name: "Natural Language Processing",       cr: 3, grade: "A",  pts: 4.0 },
      { code: "CS604",  name: "Human-Computer Interaction",        cr: 3, grade: "B+", pts: 3.3 },
      { code: "CS605",  name: "Research Methodology",              cr: 2, grade: "A",  pts: 4.0 },
    ],
  },
  {
    label: "Semester 7 — Academic Year 2024/2025",
    gpa: 3.91,
    credits: 23,
    courses: [
      { code: "CS701",  name: "Final Year Project II",             cr: 6, grade: "A",  pts: 4.0 },
      { code: "CS702",  name: "Advanced Deep Learning",            cr: 4, grade: "A",  pts: 4.0 },
      { code: "CS703",  name: "Software Architecture",             cr: 3, grade: "A-", pts: 3.7 },
      { code: "CS704",  name: "Quantum Computing Introduction",    cr: 3, grade: "A",  pts: 4.0 },
      { code: "CS705",  name: "Ethics in Artificial Intelligence", cr: 2, grade: "A",  pts: 4.0 },
      { code: "CS706",  name: "Industry Internship",               cr: 5, grade: "A",  pts: 4.0 },
    ],
  },
];

// ── PDF builder ───────────────────────────────────────────────────────────────

function buildTranscript(createDocument: any) {
  const GOLD    = "#c8a45a";
  const NAVY    = "#1a1a2e";
  const NAVY2   = "#2d3561";
  const CREAM   = "#f5f0e8";
  const LGRAY   = "#f0ede8";
  const MGRAY   = "#d8d4cc";
  const TEXT    = "#1a1a2e";

  // A4 dims
  const W = 595, H = 842;
  const ML = 48, MR = 48, MT = 155, MB = 60;
  const CW = W - ML - MR; // 499

  const doc = createDocument({
    meta: {
      title: `Official Academic Transcript — ${STUDENT.name}`,
      author: "Universitas Teknologi Nusantara",
      subject: "Official Academic Transcript",
      creator: "UTN Registrar System",
    },
    styles: {
      ".doc": { "font-family": "Helvetica", "font-size": 10, color: TEXT },
      // Table cells
      ".cell": {
        padding: [3, 5, 3, 5],
        "font-size": 9,
        "border-top-width": 0,
        "border-bottom-width": 0.5,
        "border-left-width": 0,
        "border-right-width": 0,
        "border-bottom-color": MGRAY,
        "border-bottom-style": "solid",
      },
      ".header": {
        "background-color": NAVY,
        color: CREAM,
        "font-weight": "bold",
        "font-size": 9,
      },
      ".header .cell": {
        "border-bottom-color": NAVY2,
        "border-bottom-width": 0.5,
      },
      ".row:nth-child(odd)":  { "background-color": "#ffffff" },
      ".row:nth-child(even)": { "background-color": LGRAY },
      // Custom classes
      ".section-banner":      { "background-color": NAVY2, color: CREAM, "font-weight": "bold", "font-size": 9 },
      ".section-banner .cell": { "border-bottom-color": NAVY, "border-bottom-width": 0.5 },
      ".sem-footer":          { "background-color": "#e8e4da", color: TEXT, "font-weight": "bold", "font-size": 9 },
      ".sem-footer .cell":    { "border-bottom-width": 0 },
      ".summary-row":         { "background-color": NAVY, color: GOLD, "font-weight": "bold", "font-size": 9.5 },
      ".summary-row .cell":   { "border-bottom-width": 0 },
      ".bold":                { "font-weight": "bold" },
    },
    settings: {
      margin: { top: MT, right: MR, bottom: MB, left: ML },
    },
  });

  // ── Page options for continuation pages ──────────────────────────────────
  const contPageOpts = {
    size: "A4",
    margin: { top: 55, right: MR, bottom: MB, left: ML },
    footer: { template: "OFFICIAL TRANSCRIPT  ·  Page {page} of {total}  ·  UNIVERSITAS TEKNOLOGI NUSANTARA", align: "center" },
  };

  const page1 = doc.page({
    size: "A4",
    margin: { top: MT, right: MR, bottom: MB, left: ML },
    footer: { template: "OFFICIAL TRANSCRIPT  ·  Page {page} of {total}  ·  UNIVERSITAS TEKNOLOGI NUSANTARA", align: "center" },
  });

  // ── Page 1 Header ─────────────────────────────────────────────────────────
  const HDR_Y = H - MT; // 687

  // Background
  page1.rect({ x: 0, y: HDR_Y, width: W, height: MT, fill: NAVY });
  // Gold top stripe
  page1.rect({ x: 0, y: H - 6, width: W, height: 6, fill: GOLD });
  // Gold bottom accent
  page1.rect({ x: 0, y: HDR_Y, width: W, height: 4, fill: GOLD });

  // Logo badge
  page1.rect({ x: ML, y: HDR_Y + 28, width: 64, height: 64, fill: GOLD, stroke: CREAM, strokeWidth: 1.5 });
  page1.text("UTN",    { x: ML + 10, y: HDR_Y + 62, style: { "font-family": "Helvetica-Bold", "font-size": 18, color: NAVY } });
  page1.line({ x1: ML,    y1: HDR_Y + 52, x2: ML + 64, y2: HDR_Y + 52, width: 1, color: NAVY });
  page1.text("est. 1965", { x: ML + 8,  y: HDR_Y + 37, style: { "font-size": 7, color: NAVY } });

  // University name block
  page1.text("UNIVERSITAS TEKNOLOGI NUSANTARA",
    { x: ML + 76, y: HDR_Y + 106, style: { "font-family": "Helvetica-Bold", "font-size": 13.5, color: GOLD } });
  page1.text("Faculty of Engineering and Computer Science",
    { x: ML + 76, y: HDR_Y + 91,  style: { "font-size": 9, color: "#ccbb99" } });
  page1.text("Jl. Teknologi No. 1, Jakarta Selatan 12345, Indonesia",
    { x: ML + 76, y: HDR_Y + 78,  style: { "font-size": 8, color: "#aa9977" } });
  page1.text("Tel: +62-21-1234-5678  ·  registrar@utn.ac.id  ·  www.utn.ac.id",
    { x: ML + 76, y: HDR_Y + 66,  style: { "font-size": 8, color: "#aa9977" } });

  // TRANSCRIPT title (right side)
  page1.text("OFFICIAL",    { x: 448, y: HDR_Y + 110, style: { "font-family": "Helvetica-Bold", "font-size": 10, color: CREAM } });
  page1.text("ACADEMIC",    { x: 440, y: HDR_Y + 98,  style: { "font-family": "Helvetica-Bold", "font-size": 10, color: CREAM } });
  page1.text("TRANSCRIPT",  { x: 430, y: HDR_Y + 86,  style: { "font-family": "Helvetica-Bold", "font-size": 10, color: GOLD } });
  page1.line({ x1: 428, y1: HDR_Y + 82, x2: 545, y2: HDR_Y + 82, width: 0.8, color: GOLD });
  page1.text("Issued by Registrar Office", { x: 432, y: HDR_Y + 72, style: { "font-size": 7, color: "#aa9977" } });
  page1.text(`Date: ${new Date().toLocaleDateString("en-GB", { day: "2-digit", month: "long", year: "numeric" })}`,
    { x: 432, y: HDR_Y + 61, style: { "font-size": 7, color: "#aa9977" } });

  // Horizontal separator below header
  page1.line({ x1: 0, y1: HDR_Y + 2, x2: W, y2: HDR_Y + 2, width: 0.5, color: GOLD });

  // ── Student Info (positioned just below header) ────────────────────────────
  // Draw a info box
  const IY = HDR_Y - 14; // top of info area
  page1.rect({ x: ML, y: IY - 62, width: CW, height: 64, fill: CREAM, stroke: MGRAY, strokeWidth: 0.5 });

  const col1X = ML + 8;
  const col2X = ML + 105;
  const col3X = ML + 270;
  const col4X = ML + 370;

  // Row 1
  page1.text("STUDENT NAME",  { x: col1X, y: IY - 16, style: { "font-size": 7,   color: "#888", "font-weight": "bold" } });
  page1.text(STUDENT.name,    { x: col1X, y: IY - 27, style: { "font-size": 10,  "font-family": "Helvetica-Bold", color: TEXT } });

  page1.text("STUDENT ID",    { x: col3X, y: IY - 16, style: { "font-size": 7, color: "#888", "font-weight": "bold" } });
  page1.text(STUDENT.nim,     { x: col3X, y: IY - 27, style: { "font-size": 10, "font-family": "Helvetica-Bold", color: TEXT } });

  page1.text("STATUS",        { x: col4X, y: IY - 16, style: { "font-size": 7, color: "#888", "font-weight": "bold" } });
  page1.text(STUDENT.status,  { x: col4X, y: IY - 27, style: { "font-size": 10, "font-family": "Helvetica-Bold", color: "#2d7a2d" } });

  // Divider
  page1.line({ x1: ML + 8, y1: IY - 34, x2: ML + CW - 8, y2: IY - 34, width: 0.3, color: MGRAY });

  // Row 2
  page1.text("PROGRAM",       { x: col1X, y: IY - 42, style: { "font-size": 7, color: "#888", "font-weight": "bold" } });
  page1.text(STUDENT.program, { x: col1X, y: IY - 52, style: { "font-size": 9, color: TEXT } });

  page1.text("ENTRY YEAR",        { x: col3X, y: IY - 42, style: { "font-size": 7, color: "#888", "font-weight": "bold" } });
  page1.text(String(STUDENT.entranceYear), { x: col3X, y: IY - 52, style: { "font-size": 9, color: TEXT } });

  page1.text("TOTAL CREDITS", { x: col4X, y: IY - 42, style: { "font-size": 7, color: "#888", "font-weight": "bold" } });
  page1.text(String(STUDENT.totalCredits), { x: col4X, y: IY - 52, style: { "font-size": 9, color: TEXT } });

  // Adjust cursor to below info box + small gap
  page1._yCursor = IY - 62 - 10;

  // ── Grade Tables (tableFlow for auto page-break) ──────────────────────────

  const COL_WIDTHS = [52, 210, 28, 38, 36, 48]; // Code | Course Name | SKS | Grade | Points | Quality
  const TOTAL_W = COL_WIDTHS.reduce((a, b) => a + b, 0); // should match CW
  const SCALE = CW / TOTAL_W;
  const COLS = COL_WIDTHS.map(w => Math.round(w * SCALE));

  for (const sem of SEMESTERS) {
    // Semester banner
    doc.tableFlow(
      {
        _node: {
          type: "table",
          className: "",
          columnWidths: [CW],
          columnAligns: ["left"],
          style: {},
          header: null,
          body: [{
            type: "row",
            className: "section-banner row",
            cells: [{ type: "cell", className: "cell", value: `  ${sem.label.toUpperCase()}` }],
          }],
        },
      },
      { pageOptions: contPageOpts }
    );

    // Course rows
    const tbl: any = {
      _node: {
        type: "table",
        className: "",
        columnWidths: COLS,
        columnAligns: ["left", "left", "center", "center", "center", "center"],
        style: {},
        header: {
          type: "row",
          className: "header",
          cells: [
            { type: "cell", className: "cell", value: "CODE" },
            { type: "cell", className: "cell", value: "COURSE NAME" },
            { type: "cell", className: "cell", value: "CR" },
            { type: "cell", className: "cell", value: "GRADE" },
            { type: "cell", className: "cell", value: "POINTS" },
            { type: "cell", className: "cell", value: "QUALITY" },
          ],
        },
        body: [
          ...sem.courses.map(c => ({
            type: "row",
            className: "row",
            style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: c.code },
              { type: "cell", className: "cell", value: c.name },
              { type: "cell", className: "cell", value: String(c.cr) },
              { type: "cell", className: "cell", value: c.grade },
              { type: "cell", className: "cell", value: c.pts.toFixed(2) },
              { type: "cell", className: "cell", value: (c.cr * c.pts).toFixed(2) },
            ],
          })),
          // GPA footer row
          {
            type: "row",
            className: "sem-footer row",
            style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: "" },
              { type: "cell", className: "cell", value: `Semester Credits: ${sem.credits}  ·  Semester GPA: ${sem.gpa.toFixed(2)}` },
              { type: "cell", className: "cell", value: "" },
              { type: "cell", className: "cell", value: "" },
              { type: "cell", className: "cell", value: "" },
              { type: "cell", className: "cell", value: "" },
            ],
          },
        ],
      },
    };

    doc.tableFlow(tbl, { pageOptions: contPageOpts });
    // Small spacer
    doc.textFlow(" ", { style: { "font-size": 6 }, pageOptions: contPageOpts });
  }

  // ── Cumulative Summary ────────────────────────────────────────────────────

  doc.textFlow(" ", { style: { "font-size": 4 }, pageOptions: contPageOpts });

  doc.tableFlow(
    {
      _node: {
        type: "table",
        className: "",
        columnWidths: [CW],
        columnAligns: ["left"],
        style: {},
        header: null,
        body: [{
          type: "row",
          className: "section-banner row",
          cells: [{ type: "cell", className: "cell", value: "  CUMULATIVE ACADEMIC SUMMARY" }],
        }],
      },
    },
    { pageOptions: contPageOpts }
  );

  doc.tableFlow(
    {
      _node: {
        type: "table",
        className: "",
        columnWidths: [Math.round(CW * 0.5), Math.round(CW * 0.25), Math.round(CW * 0.25)],
        columnAligns: ["left", "center", "center"],
        style: {},
        header: {
          type: "row",
          className: "header",
          cells: [
            { type: "cell", className: "cell", value: "METRIC" },
            { type: "cell", className: "cell", value: "VALUE" },
            { type: "cell", className: "cell", value: "STANDING" },
          ],
        },
        body: [
          {
            type: "row", className: "row", style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: "Cumulative GPA (Scale 4.0)" },
              { type: "cell", className: "cell", value: STUDENT.cumulativeGpa.toFixed(2) },
              { type: "cell", className: "cell", value: "Cum Laude" },
            ],
          },
          {
            type: "row", className: "row", style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: "Total Credits Earned" },
              { type: "cell", className: "cell", value: String(STUDENT.totalCredits) },
              { type: "cell", className: "cell", value: "Completed" },
            ],
          },
          {
            type: "row", className: "row", style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: "Total Semesters" },
              { type: "cell", className: "cell", value: String(SEMESTERS.length) },
              { type: "cell", className: "cell", value: `${SEMESTERS.length / 2} Years` },
            ],
          },
          {
            type: "row", className: "row", style: { height: 18 },
            cells: [
              { type: "cell", className: "cell", value: "Expected Graduation" },
              { type: "cell", className: "cell", value: "June 2025" },
              { type: "cell", className: "cell", value: "On Track" },
            ],
          },
          {
            type: "row", className: "summary-row row", style: { height: 22 },
            cells: [
              { type: "cell", className: "cell", value: "FINAL RESULT" },
              { type: "cell", className: "cell", value: `GPA ${STUDENT.cumulativeGpa.toFixed(2)} / 4.00` },
              { type: "cell", className: "cell", value: "CUM LAUDE" },
            ],
          },
        ],
      },
    },
    { pageOptions: contPageOpts }
  );

  // ── Signature Block ───────────────────────────────────────────────────────
  doc.textFlow(" ", { style: { "font-size": 10 }, pageOptions: contPageOpts });

  // Get current page and draw signatures
  const lastPage = doc._currentPage;
  if (lastPage) {
    const sigY = lastPage._yCursor - 10;
    const sig1X = ML;
    const sig2X = ML + Math.round(CW * 0.55);

    // Left: Registrar
    lastPage.line({ x1: sig1X, y1: sigY, x2: sig1X + 160, y2: sigY, width: 0.8, color: TEXT });
    lastPage.text("Dr. Siti Rahayu, M.Kom.",     { x: sig1X, y: sigY - 13, style: { "font-size": 9, "font-weight": "bold", color: TEXT } });
    lastPage.text("Head of Academic Registrar",  { x: sig1X, y: sigY - 24, style: { "font-size": 8, color: "#555" } });
    lastPage.text("Universitas Teknologi Nusantara", { x: sig1X, y: sigY - 34, style: { "font-size": 8, color: "#555" } });

    // Right: Dean
    lastPage.line({ x1: sig2X, y1: sigY, x2: sig2X + 160, y2: sigY, width: 0.8, color: TEXT });
    lastPage.text("Prof. Dr. Budi Santoso, Ph.D.", { x: sig2X, y: sigY - 13, style: { "font-size": 9, "font-weight": "bold", color: TEXT } });
    lastPage.text("Dean, Faculty of Engineering",  { x: sig2X, y: sigY - 24, style: { "font-size": 8, color: "#555" } });
    lastPage.text("Universitas Teknologi Nusantara", { x: sig2X, y: sigY - 34, style: { "font-size": 8, color: "#555" } });

    // Stamp placeholder
    lastPage.rect({ x: sig2X + 170, y: sigY - 40, width: 60, height: 60, fill: "transparent", stroke: GOLD, strokeWidth: 1 });
    lastPage.text("OFFICIAL", { x: sig2X + 180, y: sigY - 13, style: { "font-size": 7, color: GOLD } });
    lastPage.text("SEAL",     { x: sig2X + 185, y: sigY - 22, style: { "font-size": 7, color: GOLD } });

    // Verification note
    lastPage.text(
      `Verification ID: UTN-TR-${STUDENT.nim}-2025  ·  This document is officially issued by the Registrar Office.`,
      { x: ML, y: sigY - 52, style: { "font-size": 7, color: "#888" } }
    );
    lastPage.line({ x1: ML, y1: sigY - 57, x2: W - MR, y2: sigY - 57, width: 0.3, color: MGRAY });
    lastPage.text(
      "Any alteration or tampering of this document is a criminal offense under Indonesian Law No. 43/2009.",
      { x: ML, y: sigY - 66, style: { "font-size": 7, color: "#aaa" } }
    );
  }

  return doc;
}

// ── React Page ────────────────────────────────────────────────────────────────

export default function TranscriptDemoPage({ input }: { input: any }) {
  const [generating, setGenerating] = useState(false);
  const [pdfUrl, setPdfUrl] = useState<string | null>(null);
  const [elapsedMs, setElapsedMs] = useState<number | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const handleGenerate = useCallback(async () => {
    setGenerating(true);
    setErr(null);
    if (pdfUrl) {
      URL.revokeObjectURL(pdfUrl);
      setPdfUrl(null);
    }

    try {
      const t0 = performance.now();
      const lib: any = await import(location.origin + "/assets/libraries/zeb/pdf/0.1/runtime/pdf.bundle.mjs");
      const doc = buildTranscript(lib.createDocument);
      const blob = doc.toBlob();
      const url = URL.createObjectURL(blob);
      setElapsedMs(Math.round((performance.now() - t0) * 10) / 10);
      setPdfUrl(url);
    } catch (e: any) {
      setErr(e?.message ?? String(e));
    } finally {
      setGenerating(false);
    }
  }, [pdfUrl]);

  return (
    <div className="min-h-screen bg-[#0f0f1a] text-white font-sans">
      {/* Header */}
      <div className="bg-[#1a1a2e] border-b border-[#c8a45a]/30 px-8 py-4 flex items-center gap-4">
        <div className="w-10 h-10 bg-[#c8a45a] flex items-center justify-center rounded text-[#1a1a2e] font-black text-sm">
          UTN
        </div>
        <div>
          <div className="text-[#c8a45a] font-bold text-sm tracking-widest">UNIVERSITAS TEKNOLOGI NUSANTARA</div>
          <div className="text-xs text-gray-400">Academic Transcript Generator  ·  zeb/pdf demo</div>
        </div>
        <div className="ml-auto text-xs text-gray-500 font-mono">
          {SEMESTERS.length} semesters  ·  {STUDENT.totalCredits} credits  ·  {SEMESTERS.reduce((s, x) => s + x.courses.length, 0)} courses
        </div>
      </div>

      <div className="max-w-5xl mx-auto px-8 py-8 space-y-6">
        {/* Student Card */}
        <div className="bg-[#1a1a2e] border border-[#c8a45a]/20 rounded-lg overflow-hidden">
          <div className="bg-gradient-to-r from-[#c8a45a]/20 to-transparent px-6 py-3 border-b border-[#c8a45a]/20">
            <span className="text-[#c8a45a] text-xs font-bold tracking-widest">STUDENT RECORD</span>
          </div>
          <div className="px-6 py-5 grid grid-cols-3 gap-6">
            <div className="col-span-1">
              <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Full Name</div>
              <div className="font-bold text-lg text-white">{STUDENT.name}</div>
            </div>
            <div>
              <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Student ID</div>
              <div className="font-mono text-[#c8a45a] font-semibold">{STUDENT.nim}</div>
            </div>
            <div>
              <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Status</div>
              <div className="text-green-400 font-semibold">{STUDENT.status}</div>
            </div>
            <div className="col-span-2">
              <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Program</div>
              <div className="text-gray-200">{STUDENT.program}</div>
              <div className="text-gray-400 text-sm">{STUDENT.faculty}</div>
            </div>
            <div className="flex gap-8">
              <div>
                <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Cumulative GPA</div>
                <div className="text-2xl font-black text-[#c8a45a]">{STUDENT.cumulativeGpa.toFixed(2)}</div>
                <div className="text-xs text-gray-500">/ 4.00 · Cum Laude</div>
              </div>
              <div>
                <div className="text-[10px] text-gray-500 uppercase tracking-widest mb-1">Credits</div>
                <div className="text-2xl font-black text-white">{STUDENT.totalCredits}</div>
                <div className="text-xs text-gray-500">SKS earned</div>
              </div>
            </div>
          </div>
        </div>

        {/* Semester Summary */}
        <div className="bg-[#1a1a2e] border border-white/10 rounded-lg overflow-hidden">
          <div className="px-6 py-3 border-b border-white/10">
            <span className="text-xs text-gray-400 font-bold tracking-widest">SEMESTER OVERVIEW</span>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-white/5">
                  <th className="text-left px-4 py-2 text-[10px] text-gray-500 uppercase tracking-widest font-normal">Period</th>
                  <th className="text-center px-4 py-2 text-[10px] text-gray-500 uppercase tracking-widest font-normal">Courses</th>
                  <th className="text-center px-4 py-2 text-[10px] text-gray-500 uppercase tracking-widest font-normal">Credits</th>
                  <th className="text-center px-4 py-2 text-[10px] text-gray-500 uppercase tracking-widest font-normal">GPA</th>
                </tr>
              </thead>
              <tbody>
                {SEMESTERS.map((sem, i) => (
                  <tr key={i} className={i % 2 === 0 ? "bg-white/2" : ""}>
                    <td className="px-4 py-2 text-gray-300 text-xs">{sem.label}</td>
                    <td className="px-4 py-2 text-center text-gray-400 text-xs">{sem.courses.length}</td>
                    <td className="px-4 py-2 text-center text-gray-400 text-xs">{sem.credits}</td>
                    <td className="px-4 py-2 text-center">
                      <span className={`font-mono font-bold text-sm ${sem.gpa >= 3.7 ? "text-[#c8a45a]" : sem.gpa >= 3.0 ? "text-green-400" : "text-orange-400"}`}>
                        {sem.gpa.toFixed(2)}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Generate Button */}
        <div className="flex items-center gap-4">
          <button
            onClick={handleGenerate}
            disabled={generating}
            className="px-6 py-3 bg-[#c8a45a] hover:bg-[#d4b06a] disabled:opacity-50 disabled:cursor-not-allowed text-[#1a1a2e] font-bold rounded text-sm tracking-wide transition-colors"
          >
            {generating ? "Generating PDF…" : "Generate Transcript PDF"}
          </button>

          {pdfUrl && (
            <a
              href={pdfUrl}
              download={`transcript-${STUDENT.nim}.pdf`}
              className="px-6 py-3 border border-[#c8a45a]/40 hover:border-[#c8a45a] text-[#c8a45a] font-bold rounded text-sm tracking-wide transition-colors"
            >
              Download PDF
            </a>
          )}

          {elapsedMs !== null && (
            <span className="text-xs text-gray-500 font-mono">rendered in {elapsedMs}ms</span>
          )}
        </div>

        {err && (
          <div className="bg-red-900/30 border border-red-500/40 rounded p-4 font-mono text-xs text-red-300 whitespace-pre-wrap">
            {err}
          </div>
        )}

        {/* PDF Preview */}
        {pdfUrl && (
          <div className="rounded-lg overflow-hidden border border-white/10">
            <div className="bg-[#1a1a2e] px-4 py-2 border-b border-white/10 flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-[#c8a45a]" />
              <span className="text-xs text-gray-400">PDF Preview  ·  {SEMESTERS.length} pages</span>
            </div>
            <iframe
              src={pdfUrl}
              className="w-full"
              style={{ height: "900px", background: "#fff" }}
              title="Transcript PDF Preview"
            />
          </div>
        )}
      </div>
    </div>
  );
}
