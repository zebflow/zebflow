import { useState, useMemo, useRef } from "zeb";

interface CounterProps { initial?: number; label?: string; }

export default function Counter({ initial = 0, label = "Counter" }: CounterProps) {
  const [count, setCount] = useState(initial);
  const double = useMemo(() => count * 2, [count]);
  const triple = useMemo(() => count * 3, [count]);
  const clickRef = useRef(0);

  return (
    <div class="counter" data-label={label}>
      <h3>{label}</h3>
      <p class="count">Count: {count}</p>
      <p class="double">Double: {double}</p>
      <p class="triple">Triple: {triple}</p>
      <button onClick={() => { (clickRef.current as any)++; setCount(count + 1); }}>+</button>
      <button onClick={() => setCount(Math.max(0, count - 1))}>-</button>
    </div>
  );
}
