import { useState, useEffect, useRef, useMemo, usePageState, useNavigate, Link } from "zeb";
import Navbar from "@/components/navbar";
import Counter from "@/components/counter";

export default function DxTestPage(input: any) {
  const user = input?.user || "guest";
  const navigate = useNavigate();
  const [active, setActive] = useState(false);
  const headerRef = useRef<any>(null);
  const greeting = useMemo(() => `Hello, ${user}!`, [user]);
  const ps = usePageState({});

  useEffect(() => {
    ps.setPageState({ visited: true });
  }, []);

  return (
    <div class="dx-test-page">
      <Navbar currentPath="/" />
      <main>
        <h1 ref={headerRef} data-testid="greeting">{greeting}</h1>
        <Counter initial={5} label="Alpha" />
        <Counter initial={10} label="Beta" />
        <nav data-testid="links">
          <Link href="/about">About</Link>
          <Link href="/dashboard">Dashboard</Link>
        </nav>
        <button data-testid="toggle" onClick={() => setActive(!active)}>
          {active ? "On" : "Off"}
        </button>
        <button data-testid="nav-btn" onClick={() => navigate("/home")}>
          Go Home
        </button>
      </main>
    </div>
  );
}
