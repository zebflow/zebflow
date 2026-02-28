export namespace JSX {
  type Element = any;
  interface ElementClass {
    render?: any;
  }
  interface ElementAttributesProperty {
    props: any;
  }
  interface IntrinsicAttributes {
    [attr: string]: any;
  }
  interface IntrinsicElements {
    [elemName: string]: any;
  }
}

export const Fragment: any;

export function jsx(type: any, props: any, key?: any): any;
export function jsxs(type: any, props: any, key?: any): any;
export function jsxDEV(
  type: any,
  props: any,
  key: any,
  isStaticChildren: any,
  source: any,
  self: any
): any;
