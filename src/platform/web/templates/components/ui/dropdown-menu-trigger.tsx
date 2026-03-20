/**
 * DropdownMenuTrigger — passthrough wrapper kept for backward compatibility.
 * The trigger is now passed as a prop to DropdownMenu directly; this component
 * simply renders its children unchanged.
 */
export default function DropdownMenuTrigger({ children }) {
  return children;
}
