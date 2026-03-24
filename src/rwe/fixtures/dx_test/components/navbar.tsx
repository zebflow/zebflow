import { useState, useNavigate, Link } from "zeb";

interface NavbarProps { currentPath?: string; }

export default function Navbar({ currentPath = "/" }: NavbarProps) {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);

  return (
    <nav data-testid="navbar">
      <Link href="/" class={currentPath === "/" ? "active" : ""}>Home</Link>
      <Link href="/about" class={currentPath === "/about" ? "active" : ""}>About</Link>
      <button data-testid="hamburger" onClick={() => setOpen(!open)}>
        {open ? "Close" : "Menu"}
      </button>
      {open && (
        <ul data-testid="mobile-menu">
          <li><Link href="/">Home</Link></li>
          <li onClick={() => navigate("/contact")}>Contact</li>
        </ul>
      )}
    </nav>
  );
}
