// zeb/icons 0.1 — Lucide-style SVG icon components for RWE templates
//
// h is globalThis.h — set by the RWE preact bootstrap before this bundle loads.
// Each export is a component: <Search size={20} color="red" strokeWidth={1.5} className="..." />

const h = globalThis.h;

// Factory — data is an array of [tag, attrs] tuples describing SVG child elements.
function icon(data, displayName) {
  function Icon(props) {
    const size = props.size ?? 24;
    const color = props.color ?? 'currentColor';
    const sw = props.strokeWidth ?? 2;
    const cls = props.className ?? props.class ?? '';
    const rest = {};
    for (const k in props) {
      if (k !== 'size' && k !== 'color' && k !== 'strokeWidth' && k !== 'className' && k !== 'class') {
        rest[k] = props[k];
      }
    }
    return h('svg', Object.assign({
      xmlns: 'http://www.w3.org/2000/svg',
      width: size, height: size,
      viewBox: '0 0 24 24',
      fill: 'none',
      stroke: color,
      strokeWidth: sw,
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      className: cls,
    }, rest), data.map(([tag, attrs]) => h(tag, attrs)));
  }
  Icon.displayName = displayName;
  return Icon;
}

// ── Navigation ─────────────────────────────────────────────────────────────

export const ChevronLeft    = icon([['path',{d:'m15 18-6-6 6-6'}]], 'ChevronLeft');
export const ChevronRight   = icon([['path',{d:'m9 18 6-6-6-6'}]], 'ChevronRight');
export const ChevronDown    = icon([['path',{d:'m6 9 6 6 6-6'}]], 'ChevronDown');
export const ChevronUp      = icon([['path',{d:'m18 15-6-6-6 6'}]], 'ChevronUp');
export const ChevronsLeft   = icon([['path',{d:'m11 17-5-5 5-5'}],['path',{d:'m18 17-5-5 5-5'}]], 'ChevronsLeft');
export const ChevronsRight  = icon([['path',{d:'m13 17 5-5-5-5'}],['path',{d:'m6 17 5-5-5-5'}]], 'ChevronsRight');
export const ChevronsUpDown = icon([['path',{d:'m7 15 5 5 5-5'}],['path',{d:'m7 9 5-5 5 5'}]], 'ChevronsUpDown');
export const ArrowLeft      = icon([['path',{d:'M19 12H5'}],['path',{d:'m12 19-7-7 7-7'}]], 'ArrowLeft');
export const ArrowRight     = icon([['path',{d:'M5 12h14'}],['path',{d:'m12 5 7 7-7 7'}]], 'ArrowRight');
export const ArrowUp        = icon([['path',{d:'M12 19V5'}],['path',{d:'m5 12 7-7 7 7'}]], 'ArrowUp');
export const ArrowDown      = icon([['path',{d:'M12 5v14'}],['path',{d:'m19 12-7 7-7-7'}]], 'ArrowDown');

// ── Actions ────────────────────────────────────────────────────────────────

export const Plus          = icon([['path',{d:'M5 12h14'}],['path',{d:'M12 5v14'}]], 'Plus');
export const Minus         = icon([['path',{d:'M5 12h14'}]], 'Minus');
export const X             = icon([['path',{d:'M18 6 6 18'}],['path',{d:'m6 6 12 12'}]], 'X');
export const Check         = icon([['path',{d:'M20 6 9 17l-5-5'}]], 'Check');
export const Search        = icon([['circle',{cx:11,cy:11,r:8}],['path',{d:'m21 21-4.35-4.35'}]], 'Search');
export const Filter        = icon([['polygon',{points:'22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3'}]], 'Filter');
export const RefreshCw     = icon([['path',{d:'M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8'}],['path',{d:'M21 3v5h-5'}],['path',{d:'M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16'}],['path',{d:'M8 16H3v5'}]], 'RefreshCw');
export const Pencil        = icon([['path',{d:'M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z'}],['path',{d:'m15 5 4 4'}]], 'Pencil');
export const Trash2        = icon([['path',{d:'M3 6h18'}],['path',{d:'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6'}],['path',{d:'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2'}],['line',{x1:10,y1:11,x2:10,y2:17}],['line',{x1:14,y1:11,x2:14,y2:17}]], 'Trash2');
export const Copy          = icon([['rect',{width:13,height:13,x:9,y:9,rx:2,ry:2}],['path',{d:'M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1'}]], 'Copy');
export const Clipboard     = icon([['rect',{width:8,height:4,x:8,y:2,rx:1,ry:1}],['path',{d:'M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2'}]], 'Clipboard');
export const Save          = icon([['path',{d:'M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z'}],['path',{d:'M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7'}],['path',{d:'M7 3v4a1 1 0 0 0 1 1h7'}]], 'Save');
export const Download      = icon([['path',{d:'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4'}],['polyline',{points:'7 10 12 15 17 10'}],['line',{x1:12,y1:15,x2:12,y2:3}]], 'Download');
export const Upload        = icon([['path',{d:'M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4'}],['polyline',{points:'17 8 12 3 7 8'}],['line',{x1:12,y1:3,x2:12,y2:15}]], 'Upload');
export const ExternalLink  = icon([['path',{d:'M15 3h6v6'}],['path',{d:'M10 14 21 3'}],['path',{d:'M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6'}]], 'ExternalLink');
export const Undo2         = icon([['path',{d:'M9 14 4 9l5-5'}],['path',{d:'M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5v0a5.5 5.5 0 0 1-5.5 5.5H11'}]], 'Undo2');
export const Redo2         = icon([['path',{d:'m15 14 5-5-5-5'}],['path',{d:'M20 9H9.5A5.5 5.5 0 0 0 4 14.5v0A5.5 5.5 0 0 0 9.5 20H13'}]], 'Redo2');

// ── UI State ───────────────────────────────────────────────────────────────

export const Eye           = icon([['path',{d:'M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z'}],['circle',{cx:12,cy:12,r:3}]], 'Eye');
export const EyeOff        = icon([['path',{d:'M9.88 9.88a3 3 0 1 0 4.24 4.24'}],['path',{d:'M10.73 5.08A10.43 10.43 0 0 1 12 5c7 0 10 7 10 7a13.16 13.16 0 0 1-1.67 2.68'}],['path',{d:'M6.61 6.61A13.526 13.526 0 0 0 2 12s3 7 10 7a9.74 9.74 0 0 0 5.39-1.61'}],['line',{x1:2,y1:2,x2:22,y2:22}]], 'EyeOff');
export const Lock          = icon([['rect',{width:18,height:11,x:3,y:11,rx:2,ry:2}],['path',{d:'M7 11V7a5 5 0 0 1 10 0v4'}]], 'Lock');
export const Unlock        = icon([['rect',{width:18,height:11,x:3,y:11,rx:2,ry:2}],['path',{d:'M7 11V7a5 5 0 0 1 9.9-1'}]], 'Unlock');
export const Settings      = icon([['path',{d:'M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z'}],['circle',{cx:12,cy:12,r:3}]], 'Settings');
export const Menu          = icon([['line',{x1:4,y1:12,x2:20,y2:12}],['line',{x1:4,y1:6,x2:20,y2:6}],['line',{x1:4,y1:18,x2:20,y2:18}]], 'Menu');
export const MoreHorizontal= icon([['circle',{cx:12,cy:12,r:1}],['circle',{cx:19,cy:12,r:1}],['circle',{cx:5,cy:12,r:1}]], 'MoreHorizontal');
export const MoreVertical  = icon([['circle',{cx:12,cy:12,r:1}],['circle',{cx:12,cy:5,r:1}],['circle',{cx:12,cy:19,r:1}]], 'MoreVertical');
export const Maximize2     = icon([['polyline',{points:'15 3 21 3 21 9'}],['polyline',{points:'9 21 3 21 3 15'}],['line',{x1:21,y1:3,x2:14,y2:10}],['line',{x1:3,y1:21,x2:10,y2:14}]], 'Maximize2');
export const Minimize2     = icon([['polyline',{points:'4 14 10 14 10 20'}],['polyline',{points:'20 10 14 10 14 4'}],['line',{x1:10,y1:14,x2:3,y2:21}],['line',{x1:21,y1:3,x2:14,y2:10}]], 'Minimize2');
export const PanelLeft     = icon([['rect',{width:18,height:18,x:3,y:3,rx:2}],['path',{d:'M9 3v18'}]], 'PanelLeft');
export const PanelRight    = icon([['rect',{width:18,height:18,x:3,y:3,rx:2}],['path',{d:'M15 3v18'}]], 'PanelRight');
export const SidebarOpen   = icon([['rect',{width:18,height:18,x:3,y:3,rx:2}],['path',{d:'M9 3v18'}],['path',{d:'m14 9 3 3-3 3'}]], 'SidebarOpen');
export const SidebarClose  = icon([['rect',{width:18,height:18,x:3,y:3,rx:2}],['path',{d:'M9 3v18'}],['path',{d:'m16 15-3-3 3-3'}]], 'SidebarClose');

// ── Status / Feedback ──────────────────────────────────────────────────────

export const AlertCircle   = icon([['circle',{cx:12,cy:12,r:10}],['line',{x1:12,y1:8,x2:12,y2:12}],['line',{x1:12,y1:16,x2:'12.01',y2:16}]], 'AlertCircle');
export const AlertTriangle = icon([['path',{d:'m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z'}],['path',{d:'M12 9v4'}],['path',{d:'M12 17h.01'}]], 'AlertTriangle');
export const Info          = icon([['circle',{cx:12,cy:12,r:10}],['path',{d:'M12 16v-4'}],['path',{d:'M12 8h.01'}]], 'Info');
export const CheckCircle   = icon([['path',{d:'M22 11.08V12a10 10 0 1 1-5.93-9.14'}],['polyline',{points:'22 4 12 14.01 9 11.01'}]], 'CheckCircle');
export const CheckCircle2  = icon([['circle',{cx:12,cy:12,r:10}],['path',{d:'m9 12 2 2 4-4'}]], 'CheckCircle2');
export const XCircle       = icon([['circle',{cx:12,cy:12,r:10}],['path',{d:'m15 9-6 6'}],['path',{d:'m9 9 6 6'}]], 'XCircle');
export const Loader2       = icon([['path',{d:'M21 12a9 9 0 1 1-6.219-8.56'}]], 'Loader2');

// ── Data / Content ─────────────────────────────────────────────────────────

export const Database      = icon([['ellipse',{cx:12,cy:5,rx:9,ry:3}],['path',{d:'M3 5V19a9 3 0 0 0 18 0V5'}],['path',{d:'M3 12a9 3 0 0 0 18 0'}]], 'Database');
export const TableIcon     = icon([['rect',{width:18,height:18,x:3,y:3,rx:2,ry:2}],['path',{d:'M3 9h18'}],['path',{d:'M3 15h18'}],['path',{d:'M9 3v18'}]], 'TableIcon');
export const Columns2      = icon([['rect',{width:18,height:18,x:3,y:3,rx:2}],['path',{d:'M12 3v18'}]], 'Columns2');
export const BarChart2     = icon([['line',{x1:18,y1:20,x2:18,y2:10}],['line',{x1:12,y1:20,x2:12,y2:4}],['line',{x1:6,y1:20,x2:6,y2:14}]], 'BarChart2');
export const PieChart      = icon([['path',{d:'M21.21 15.89A10 10 0 1 1 8 2.83'}],['path',{d:'M22 12A10 10 0 0 0 12 2v10z'}]], 'PieChart');
export const TrendingUp    = icon([['polyline',{points:'22 7 13.5 15.5 8.5 10.5 2 17'}],['polyline',{points:'16 7 22 7 22 13'}]], 'TrendingUp');
export const TrendingDown  = icon([['polyline',{points:'22 17 13.5 8.5 8.5 13.5 2 7'}],['polyline',{points:'16 17 22 17 22 11'}]], 'TrendingDown');

// ── Files ──────────────────────────────────────────────────────────────────

export const File          = icon([['path',{d:'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z'}],['path',{d:'M14 2v4a2 2 0 0 0 2 2h4'}]], 'File');
export const FileText      = icon([['path',{d:'M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z'}],['path',{d:'M14 2v4a2 2 0 0 0 2 2h4'}],['path',{d:'M10 9H8'}],['path',{d:'M16 13H8'}],['path',{d:'M16 17H8'}]], 'FileText');
export const Folder        = icon([['path',{d:'M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z'}]], 'Folder');
export const FolderOpen    = icon([['path',{d:'m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2'}]], 'FolderOpen');
export const Code2         = icon([['path',{d:'m18 16 4-4-4-4'}],['path',{d:'m6 8-4 4 4 4'}],['path',{d:'m14.5 4-5 16'}]], 'Code2');
export const Terminal      = icon([['polyline',{points:'4 17 10 11 4 5'}],['line',{x1:12,y1:19,x2:20,y2:19}]], 'Terminal');

// ── People / Auth ──────────────────────────────────────────────────────────

export const User          = icon([['circle',{cx:12,cy:8,r:4}],['path',{d:'M20 21a8 8 0 1 0-16 0'}]], 'User');
export const Users         = icon([['path',{d:'M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2'}],['circle',{cx:9,cy:7,r:4}],['path',{d:'M22 21v-2a4 4 0 0 0-3-3.87'}],['path',{d:'M16 3.13a4 4 0 0 1 0 7.75'}]], 'Users');
export const KeyRound      = icon([['circle',{cx:7.5,cy:15.5,r:5.5}],['path',{d:'m21 2-9.6 9.6'}],['path',{d:'m15.5 7.5 3 3L22 7l-3-3'}]], 'KeyRound');
export const LogIn         = icon([['path',{d:'M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4'}],['polyline',{points:'10 17 15 12 10 7'}],['line',{x1:15,y1:12,x2:3,y2:12}]], 'LogIn');
export const LogOut        = icon([['path',{d:'M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4'}],['polyline',{points:'16 17 21 12 16 7'}],['line',{x1:21,y1:12,x2:9,y2:12}]], 'LogOut');

// ── Misc ───────────────────────────────────────────────────────────────────

export const Globe         = icon([['circle',{cx:12,cy:12,r:10}],['path',{d:'M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z'}],['path',{d:'M2 12h20'}]], 'Globe');
export const Package       = icon([['path',{d:'m7.5 4.27 9 5.15'}],['path',{d:'M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z'}],['path',{d:'m3.3 7 8.7 5 8.7-5'}],['path',{d:'M12 22V12'}]], 'Package');
export const Zap           = icon([['polygon',{points:'13 2 3 14 12 14 11 22 21 10 12 10 13 2'}]], 'Zap');
export const Star          = icon([['polygon',{points:'12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2'}]], 'Star');
export const Layers        = icon([['path',{d:'m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z'}],['path',{d:'m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65'}],['path',{d:'m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65'}]], 'Layers');
export const LayoutGrid    = icon([['rect',{width:7,height:7,x:3,y:3,rx:1}],['rect',{width:7,height:7,x:14,y:3,rx:1}],['rect',{width:7,height:7,x:14,y:14,rx:1}],['rect',{width:7,height:7,x:3,y:14,rx:1}]], 'LayoutGrid');
export const ListIcon      = icon([['line',{x1:8,y1:6,x2:21,y2:6}],['line',{x1:8,y1:12,x2:21,y2:12}],['line',{x1:8,y1:18,x2:21,y2:18}],['line',{x1:3,y1:6,x2:'3.01',y2:6}],['line',{x1:3,y1:12,x2:'3.01',y2:12}],['line',{x1:3,y1:18,x2:'3.01',y2:18}]], 'ListIcon');
export const Cpu           = icon([['rect',{width:16,height:16,x:4,y:4,rx:2}],['rect',{width:6,height:6,x:9,y:9,rx:1}],['path',{d:'M15 2v2'}],['path',{d:'M15 20v2'}],['path',{d:'M2 15h2'}],['path',{d:'M2 9h2'}],['path',{d:'M20 15h2'}],['path',{d:'M20 9h2'}],['path',{d:'M9 2v2'}],['path',{d:'M9 20v2'}]], 'Cpu');
export const Cloud         = icon([['path',{d:'M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z'}]], 'Cloud');
export const Wifi          = icon([['path',{d:'M5 12.55a11 11 0 0 1 14.08 0'}],['path',{d:'M1.42 9a16 16 0 0 1 21.16 0'}],['path',{d:'M8.53 16.11a6 6 0 0 1 6.95 0'}],['line',{x1:12,y1:20,x2:'12.01',y2:20}]], 'Wifi');
export const Bell          = icon([['path',{d:'M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9'}],['path',{d:'M10.3 21a1.94 1.94 0 0 0 3.4 0'}]], 'Bell');
export const BellOff       = icon([['path',{d:'M8.7 3A6 6 0 0 1 18 8a21.3 21.3 0 0 0 .6 5'}],['path',{d:'M17 17H3s3-2 3-9a4.67 4.67 0 0 1 .3-1.7'}],['path',{d:'M10.3 21a1.94 1.94 0 0 0 3.4 0'}],['line',{x1:2,y1:2,x2:22,y2:22}]], 'BellOff');
export const Tag           = icon([['path',{d:'M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z'}],['circle',{cx:7.5,cy:7.5,r:.5,fill:'currentColor'}]], 'Tag');
export const Bookmark      = icon([['path',{d:'m19 21-7-4-7 4V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v16z'}]], 'Bookmark');
export const Hash          = icon([['line',{x1:4,y1:9,x2:20,y2:9}],['line',{x1:4,y1:15,x2:20,y2:15}],['line',{x1:10,y1:3,x2:8,y2:21}],['line',{x1:16,y1:3,x2:14,y2:21}]], 'Hash');
export const Slash         = icon([['line',{x1:22,y1:2,x2:2,y2:22}]], 'Slash');
export const Sparkles      = icon([['path',{d:'m12 3-1.912 5.813a2 2 0 0 1-1.275 1.275L3 12l5.813 1.912a2 2 0 0 1 1.275 1.275L12 21l1.912-5.813a2 2 0 0 1 1.275-1.275L21 12l-5.813-1.912a2 2 0 0 1-1.275-1.275L12 3Z'}],['path',{d:'M5 3v4'}],['path',{d:'M19 17v4'}],['path',{d:'M3 5h4'}],['path',{d:'M17 19h4'}]], 'Sparkles');

// ── Devicons — developer icon CSS helpers ─────────────────────────────────
// CSS classes for devicons font icons. Requires the devicons stylesheet.
// ensureDevicons() injects it on demand if not already in <head>.

const _DEVICONS_CSS = '/assets/libraries/zeb/icons/0.1/runtime/devicons.css';

export function ensureDevicons(options = {}) {
  if (typeof document === 'undefined') return false;
  const href = String(options.href || _DEVICONS_CSS);
  const marker = `link[data-zeb-devicons='${href}']`;
  if (document.head.querySelector(marker)) return true;
  const link = document.createElement('link');
  link.rel = 'stylesheet';
  link.href = href;
  link.setAttribute('data-zeb-devicons', href);
  document.head.appendChild(link);
  return true;
}

export function dbKindIconClass(kind) {
  switch (String(kind || '').trim().toLowerCase()) {
    case 'postgresql': case 'postgres': case 'pg': return 'devicon-postgresql-plain colored';
    case 'mysql':   return 'devicon-mysql-plain colored';
    case 'sqlite':  return 'devicon-sqlite-plain colored';
    case 'redis':   return 'devicon-redis-plain colored';
    case 'mongodb': return 'devicon-mongodb-plain colored';
    case 'qdrant':  return 'devicon-vectorlogozone-plain';
    case 'sjtable': case 'sekejap': return 'zf-icon-sjtable';
    default: return 'zf-icon-default-db';
  }
}

export function dbObjectIconClass(kind) {
  switch (String(kind || '').trim().toLowerCase()) {
    case 'schema':   return 'zf-icon-schema';
    case 'table':    return 'zf-icon-table';
    case 'function': return 'zf-icon-function';
    case 'file':     return 'zf-icon-file';
    case 'folder':   return 'zf-icon-folder';
    case 'node':     return 'zf-icon-node';
    default: return 'zf-icon-default-db';
  }
}
