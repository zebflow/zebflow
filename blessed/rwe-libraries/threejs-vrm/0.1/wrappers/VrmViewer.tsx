/**
 * VrmViewer — RWE wrapper for VRM avatar rendering.
 *
 * IMPORT in TSX page:
 *   import VrmViewer from "zeb/threejs-vrm";
 *
 * ─── Basic usage ──────────────────────────────────────────────────────────
 *   <VrmViewer
 *     modelUrl="/assets/my-avatar.vrm"
 *     height="500px"
 *     autoRotate
 *   />
 *
 * ─── With event handler (imperative post-mount) ───────────────────────────
 *   <VrmViewer
 *     id="my-vrm"
 *     modelUrl="/assets/avatar.vrm"
 *     height="400px"
 *     background="#1e293b"
 *   />
 *   // then in behavior.ts:
 *   document.getElementById("my-vrm").addEventListener("zeb:vrm:ready", (e) => {
 *     const { instance } = e.detail;
 *     instance.setExpression("happy", 1.0);
 *   });
 *
 * ─── Imperative registry ──────────────────────────────────────────────────
 *   const inst = window.__zebVrm.get("my-vrm");
 *   inst.setExpression("happy", 1.0);
 */
export const app = {};

export default function VrmViewer(props) {
  const config = JSON.stringify({
    modelUrl:         props.modelUrl || props.model_url || "",
    height:           props.height || "400px",
    background:       props.background || "transparent",
    autoRotate:       props.autoRotate ?? false,
    cameraZ:          props.cameraZ ?? 1.5,
    ambientColor:     props.ambientColor,
    ambientIntensity: props.ambientIntensity,
    dirColor:         props.dirColor,
    dirIntensity:     props.dirIntensity,
  });

  return (
    <div
      data-zeb-lib="threejs-vrm"
      data-zeb-wrapper="VrmViewer"
      data-config={config}
      id={props.id}
      className={props.className}
      style={{ width: "100%", height: props.height || "400px" }}
    />
  );
}
