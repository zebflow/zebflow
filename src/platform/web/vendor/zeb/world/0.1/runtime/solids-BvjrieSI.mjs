var e=Object.defineProperty,t=(t,n)=>{let r={};for(var i in t)e(r,i,{get:t[i],enumerable:!0});return n||e(r,Symbol.toStringTag,{value:`Module`}),r};function n(e,t){return class extends e{constructor(...e){super(...e),t(this)}}}const r=n(Array,e=>e.fill(0));let i=1e-6;function a(e){function t(t=0,n=0){let r=new e(2);return t!==void 0&&(r[0]=t,n!==void 0&&(r[1]=n)),r}let n=t;function r(t,n,r){let i=r??new e(2);return i[0]=t,i[1]=n,i}function a(t,n){let r=n??new e(2);return r[0]=Math.ceil(t[0]),r[1]=Math.ceil(t[1]),r}function o(t,n){let r=n??new e(2);return r[0]=Math.floor(t[0]),r[1]=Math.floor(t[1]),r}function s(t,n){let r=n??new e(2);return r[0]=Math.round(t[0]),r[1]=Math.round(t[1]),r}function c(t,n=0,r=1,i){let a=i??new e(2);return a[0]=Math.min(r,Math.max(n,t[0])),a[1]=Math.min(r,Math.max(n,t[1])),a}function l(t,n,r){let i=r??new e(2);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i}function u(t,n,r,i){let a=i??new e(2);return a[0]=t[0]+n[0]*r,a[1]=t[1]+n[1]*r,a}function d(e,t){let n=e[0],r=e[1],i=t[0],a=t[1],o=Math.sqrt(n*n+r*r)*Math.sqrt(i*i+a*a),s=o&&E(e,t)/o;return Math.acos(s)}function f(t,n,r){let i=r??new e(2);return i[0]=t[0]-n[0],i[1]=t[1]-n[1],i}let p=f;function m(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i}function h(e,t){return e[0]===t[0]&&e[1]===t[1]}function g(t,n,r,i){let a=i??new e(2);return a[0]=t[0]+r*(n[0]-t[0]),a[1]=t[1]+r*(n[1]-t[1]),a}function _(t,n,r,i){let a=i??new e(2);return a[0]=t[0]+r[0]*(n[0]-t[0]),a[1]=t[1]+r[1]*(n[1]-t[1]),a}function v(t,n,r){let i=r??new e(2);return i[0]=Math.max(t[0],n[0]),i[1]=Math.max(t[1],n[1]),i}function y(t,n,r){let i=r??new e(2);return i[0]=Math.min(t[0],n[0]),i[1]=Math.min(t[1],n[1]),i}function b(t,n,r){let i=r??new e(2);return i[0]=t[0]*n,i[1]=t[1]*n,i}let x=b;function S(t,n,r){let i=r??new e(2);return i[0]=t[0]/n,i[1]=t[1]/n,i}function C(t,n){let r=n??new e(2);return r[0]=1/t[0],r[1]=1/t[1],r}let w=C;function T(t,n,r){let i=r??new e(3),a=t[0]*n[1]-t[1]*n[0];return i[0]=0,i[1]=0,i[2]=a,i}function E(e,t){return e[0]*t[0]+e[1]*t[1]}function D(e){let t=e[0],n=e[1];return Math.sqrt(t*t+n*n)}let O=D;function k(e){let t=e[0],n=e[1];return t*t+n*n}let A=k;function j(e,t){let n=e[0]-t[0],r=e[1]-t[1];return Math.sqrt(n*n+r*r)}let M=j;function N(e,t){let n=e[0]-t[0],r=e[1]-t[1];return n*n+r*r}let P=N;function F(t,n){let r=n??new e(2),i=t[0],a=t[1],o=i*i+a*a,s=o>0?1/Math.sqrt(o):1;return r[0]=i*s,r[1]=a*s,r}function I(t,n){let r=n??new e(2);return r[0]=-t[0],r[1]=-t[1],r}function ee(t,n){let r=n??new e(2);return r[0]=t[0],r[1]=t[1],r}let L=ee;function R(t,n,r){let i=r??new e(2);return i[0]=t[0]*n[0],i[1]=t[1]*n[1],i}let te=R;function ne(t,n,r){let i=r??new e(2);return i[0]=t[0]/n[0],i[1]=t[1]/n[1],i}let re=ne;function z(t=1,n){let r=n??new e(2),i=Math.random()*2*Math.PI;return r[0]=Math.cos(i)*t,r[1]=Math.sin(i)*t,r}function B(t){let n=t??new e(2);return n[0]=0,n[1]=0,n}function ie(t,n,r){let i=r??new e(2),a=t[0],o=t[1];return i[0]=a*n[0]+o*n[4]+n[12],i[1]=a*n[1]+o*n[5]+n[13],i}function ae(t,n,r){let i=r??new e(2),a=t[0],o=t[1];return i[0]=n[0]*a+n[4]*o+n[8],i[1]=n[1]*a+n[5]*o+n[9],i}function V(t,n,r,i){let a=i??new e(2),o=t[0]-n[0],s=t[1]-n[1],c=Math.sin(r),l=Math.cos(r);return a[0]=o*l-s*c+n[0],a[1]=o*c+s*l+n[1],a}function H(t,n,r){let i=r??new e(2);return F(t,i),b(i,n,i)}function oe(t,n,r){let i=r??new e(2);return D(t)>n?H(t,n,i):ee(t,i)}function se(t,n,r){return g(t,n,.5,r??new e(2))}return{create:t,fromValues:n,set:r,ceil:a,floor:o,round:s,clamp:c,add:l,addScaled:u,angle:d,subtract:f,sub:p,equalsApproximately:m,equals:h,lerp:g,lerpV:_,max:v,min:y,mulScalar:b,scale:x,divScalar:S,inverse:C,invert:w,cross:T,dot:E,length:D,len:O,lengthSq:k,lenSq:A,distance:j,dist:M,distanceSq:N,distSq:P,normalize:F,negate:I,copy:ee,clone:L,multiply:R,mul:te,divide:ne,div:re,random:z,zero:B,transformMat4:ie,transformMat3:ae,rotate:V,setLength:H,truncate:oe,midpoint:se}}const o=/* @__PURE__ */ new Map;function s(e){let t=o.get(e);return t||(t=a(e),o.set(e,t)),t}function c(e){function t(t,n,r){let i=new e(3);return t!==void 0&&(i[0]=t,n!==void 0&&(i[1]=n,r!==void 0&&(i[2]=r))),i}let n=t;function r(t,n,r,i){let a=i??new e(3);return a[0]=t,a[1]=n,a[2]=r,a}function a(t,n){let r=n??new e(3);return r[0]=Math.ceil(t[0]),r[1]=Math.ceil(t[1]),r[2]=Math.ceil(t[2]),r}function o(t,n){let r=n??new e(3);return r[0]=Math.floor(t[0]),r[1]=Math.floor(t[1]),r[2]=Math.floor(t[2]),r}function s(t,n){let r=n??new e(3);return r[0]=Math.round(t[0]),r[1]=Math.round(t[1]),r[2]=Math.round(t[2]),r}function c(t,n=0,r=1,i){let a=i??new e(3);return a[0]=Math.min(r,Math.max(n,t[0])),a[1]=Math.min(r,Math.max(n,t[1])),a[2]=Math.min(r,Math.max(n,t[2])),a}function l(t,n,r){let i=r??new e(3);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i[2]=t[2]+n[2],i}function u(t,n,r,i){let a=i??new e(3);return a[0]=t[0]+n[0]*r,a[1]=t[1]+n[1]*r,a[2]=t[2]+n[2]*r,a}function d(e,t){let n=e[0],r=e[1],i=e[2],a=t[0],o=t[1],s=t[2],c=Math.sqrt(n*n+r*r+i*i)*Math.sqrt(a*a+o*o+s*s),l=c&&E(e,t)/c;return Math.acos(l)}function f(t,n,r){let i=r??new e(3);return i[0]=t[0]-n[0],i[1]=t[1]-n[1],i[2]=t[2]-n[2],i}let p=f;function m(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i&&Math.abs(e[2]-t[2])<i}function h(e,t){return e[0]===t[0]&&e[1]===t[1]&&e[2]===t[2]}function g(t,n,r,i){let a=i??new e(3);return a[0]=t[0]+r*(n[0]-t[0]),a[1]=t[1]+r*(n[1]-t[1]),a[2]=t[2]+r*(n[2]-t[2]),a}function _(t,n,r,i){let a=i??new e(3);return a[0]=t[0]+r[0]*(n[0]-t[0]),a[1]=t[1]+r[1]*(n[1]-t[1]),a[2]=t[2]+r[2]*(n[2]-t[2]),a}function v(t,n,r){let i=r??new e(3);return i[0]=Math.max(t[0],n[0]),i[1]=Math.max(t[1],n[1]),i[2]=Math.max(t[2],n[2]),i}function y(t,n,r){let i=r??new e(3);return i[0]=Math.min(t[0],n[0]),i[1]=Math.min(t[1],n[1]),i[2]=Math.min(t[2],n[2]),i}function b(t,n,r){let i=r??new e(3);return i[0]=t[0]*n,i[1]=t[1]*n,i[2]=t[2]*n,i}let x=b;function S(t,n,r){let i=r??new e(3);return i[0]=t[0]/n,i[1]=t[1]/n,i[2]=t[2]/n,i}function C(t,n){let r=n??new e(3);return r[0]=1/t[0],r[1]=1/t[1],r[2]=1/t[2],r}let w=C;function T(t,n,r){let i=r??new e(3),a=t[2]*n[0]-t[0]*n[2],o=t[0]*n[1]-t[1]*n[0];return i[0]=t[1]*n[2]-t[2]*n[1],i[1]=a,i[2]=o,i}function E(e,t){return e[0]*t[0]+e[1]*t[1]+e[2]*t[2]}function D(e){let t=e[0],n=e[1],r=e[2];return Math.sqrt(t*t+n*n+r*r)}let O=D;function k(e){let t=e[0],n=e[1],r=e[2];return t*t+n*n+r*r}let A=k;function j(e,t){let n=e[0]-t[0],r=e[1]-t[1],i=e[2]-t[2];return Math.sqrt(n*n+r*r+i*i)}let M=j;function N(e,t){let n=e[0]-t[0],r=e[1]-t[1],i=e[2]-t[2];return n*n+r*r+i*i}let P=N;function F(t,n){let r=n??new e(3),i=t[0],a=t[1],o=t[2],s=i*i+a*a+o*o,c=s>0?1/Math.sqrt(s):1;return r[0]=i*c,r[1]=a*c,r[2]=o*c,r}function I(t,n){let r=n??new e(3);return r[0]=-t[0],r[1]=-t[1],r[2]=-t[2],r}function ee(t,n){let r=n??new e(3);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r}let L=ee;function R(t,n,r){let i=r??new e(3);return i[0]=t[0]*n[0],i[1]=t[1]*n[1],i[2]=t[2]*n[2],i}let te=R;function ne(t,n,r){let i=r??new e(3);return i[0]=t[0]/n[0],i[1]=t[1]/n[1],i[2]=t[2]/n[2],i}let re=ne;function z(t=1,n){let r=n??new e(3),i=Math.random()*2*Math.PI,a=Math.random()*2-1,o=Math.sqrt(1-a*a)*t;return r[0]=Math.cos(i)*o,r[1]=Math.sin(i)*o,r[2]=a*t,r}function B(t){let n=t??new e(3);return n[0]=0,n[1]=0,n[2]=0,n}function ie(t,n,r){let i=r??new e(3),a=t[0],o=t[1],s=t[2],c=n[3]*a+n[7]*o+n[11]*s+n[15]||1;return i[0]=(n[0]*a+n[4]*o+n[8]*s+n[12])/c,i[1]=(n[1]*a+n[5]*o+n[9]*s+n[13])/c,i[2]=(n[2]*a+n[6]*o+n[10]*s+n[14])/c,i}function ae(t,n,r){let i=r??new e(3),a=t[0],o=t[1],s=t[2];return i[0]=a*n[0]+o*n[4]+s*n[8],i[1]=a*n[1]+o*n[5]+s*n[9],i[2]=a*n[2]+o*n[6]+s*n[10],i}function V(t,n,r){let i=r??new e(3),a=t[0],o=t[1],s=t[2];return i[0]=a*n[0]+o*n[4]+s*n[8],i[1]=a*n[1]+o*n[5]+s*n[9],i[2]=a*n[2]+o*n[6]+s*n[10],i}function H(t,n,r){let i=r??new e(3),a=n[0],o=n[1],s=n[2],c=n[3]*2,l=t[0],u=t[1],d=t[2],f=o*d-s*u,p=s*l-a*d,m=a*u-o*l;return i[0]=l+f*c+(o*m-s*p)*2,i[1]=u+p*c+(s*f-a*m)*2,i[2]=d+m*c+(a*p-o*f)*2,i}function oe(t,n){let r=n??new e(3);return r[0]=t[12],r[1]=t[13],r[2]=t[14],r}function se(t,n,r){let i=r??new e(3),a=n*4;return i[0]=t[a+0],i[1]=t[a+1],i[2]=t[a+2],i}function ce(t,n){let r=n??new e(3),i=t[0],a=t[1],o=t[2],s=t[4],c=t[5],l=t[6],u=t[8],d=t[9],f=t[10];return r[0]=Math.sqrt(i*i+a*a+o*o),r[1]=Math.sqrt(s*s+c*c+l*l),r[2]=Math.sqrt(u*u+d*d+f*f),r}function le(t,n,r,i){let a=i??new e(3),o=[],s=[];return o[0]=t[0]-n[0],o[1]=t[1]-n[1],o[2]=t[2]-n[2],s[0]=o[0],s[1]=o[1]*Math.cos(r)-o[2]*Math.sin(r),s[2]=o[1]*Math.sin(r)+o[2]*Math.cos(r),a[0]=s[0]+n[0],a[1]=s[1]+n[1],a[2]=s[2]+n[2],a}function ue(t,n,r,i){let a=i??new e(3),o=[],s=[];return o[0]=t[0]-n[0],o[1]=t[1]-n[1],o[2]=t[2]-n[2],s[0]=o[2]*Math.sin(r)+o[0]*Math.cos(r),s[1]=o[1],s[2]=o[2]*Math.cos(r)-o[0]*Math.sin(r),a[0]=s[0]+n[0],a[1]=s[1]+n[1],a[2]=s[2]+n[2],a}function U(t,n,r,i){let a=i??new e(3),o=[],s=[];return o[0]=t[0]-n[0],o[1]=t[1]-n[1],o[2]=t[2]-n[2],s[0]=o[0]*Math.cos(r)-o[1]*Math.sin(r),s[1]=o[0]*Math.sin(r)+o[1]*Math.cos(r),s[2]=o[2],a[0]=s[0]+n[0],a[1]=s[1]+n[1],a[2]=s[2]+n[2],a}function de(t,n,r){let i=r??new e(3);return F(t,i),b(i,n,i)}function fe(t,n,r){let i=r??new e(3);return D(t)>n?de(t,n,i):ee(t,i)}function pe(t,n,r){return g(t,n,.5,r??new e(3))}return{create:t,fromValues:n,set:r,ceil:a,floor:o,round:s,clamp:c,add:l,addScaled:u,angle:d,subtract:f,sub:p,equalsApproximately:m,equals:h,lerp:g,lerpV:_,max:v,min:y,mulScalar:b,scale:x,divScalar:S,inverse:C,invert:w,cross:T,dot:E,length:D,len:O,lengthSq:k,lenSq:A,distance:j,dist:M,distanceSq:N,distSq:P,normalize:F,negate:I,copy:ee,clone:L,multiply:R,mul:te,divide:ne,div:re,random:z,zero:B,transformMat4:ie,transformMat4Upper3x3:ae,transformMat3:V,transformQuat:H,getTranslation:oe,getAxis:se,getScaling:ce,rotateX:le,rotateY:ue,rotateZ:U,setLength:de,truncate:fe,midpoint:pe}}const l=/* @__PURE__ */ new Map;function u(e){let t=l.get(e);return t||(t=c(e),l.set(e,t)),t}function d(e){let t=s(e),n=u(e);function r(t,n,r,i,a,o,s,c,l){let u=new e(12);return u[3]=0,u[7]=0,u[11]=0,t!==void 0&&(u[0]=t,n!==void 0&&(u[1]=n,r!==void 0&&(u[2]=r,i!==void 0&&(u[4]=i,a!==void 0&&(u[5]=a,o!==void 0&&(u[6]=o,s!==void 0&&(u[8]=s,c!==void 0&&(u[9]=c,l!==void 0&&(u[10]=l))))))))),u}function a(t,n,r,i,a,o,s,c,l,u){let d=u??new e(12);return d[0]=t,d[1]=n,d[2]=r,d[3]=0,d[4]=i,d[5]=a,d[6]=o,d[7]=0,d[8]=s,d[9]=c,d[10]=l,d[11]=0,d}function o(t,n){let r=n??new e(12);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[3]=0,r[4]=t[4],r[5]=t[5],r[6]=t[6],r[7]=0,r[8]=t[8],r[9]=t[9],r[10]=t[10],r[11]=0,r}function c(t,n){let r=n??new e(12),i=t[0],a=t[1],o=t[2],s=t[3],c=i+i,l=a+a,u=o+o,d=i*c,f=a*c,p=a*l,m=o*c,h=o*l,g=o*u,_=s*c,v=s*l,y=s*u;return r[0]=1-p-g,r[1]=f+y,r[2]=m-v,r[3]=0,r[4]=f-y,r[5]=1-d-g,r[6]=h+_,r[7]=0,r[8]=m+v,r[9]=h-_,r[10]=1-d-p,r[11]=0,r}function l(t,n){let r=n??new e(12);return r[0]=-t[0],r[1]=-t[1],r[2]=-t[2],r[4]=-t[4],r[5]=-t[5],r[6]=-t[6],r[8]=-t[8],r[9]=-t[9],r[10]=-t[10],r}function d(t,n,r){let i=r??new e(12);return i[0]=t[0]*n,i[1]=t[1]*n,i[2]=t[2]*n,i[4]=t[4]*n,i[5]=t[5]*n,i[6]=t[6]*n,i[8]=t[8]*n,i[9]=t[9]*n,i[10]=t[10]*n,i}let f=d;function p(t,n,r){let i=r??new e(12);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i[2]=t[2]+n[2],i[4]=t[4]+n[4],i[5]=t[5]+n[5],i[6]=t[6]+n[6],i[8]=t[8]+n[8],i[9]=t[9]+n[9],i[10]=t[10]+n[10],i}function m(t,n){let r=n??new e(12);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[4]=t[4],r[5]=t[5],r[6]=t[6],r[8]=t[8],r[9]=t[9],r[10]=t[10],r}let h=m;function g(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i&&Math.abs(e[2]-t[2])<i&&Math.abs(e[4]-t[4])<i&&Math.abs(e[5]-t[5])<i&&Math.abs(e[6]-t[6])<i&&Math.abs(e[8]-t[8])<i&&Math.abs(e[9]-t[9])<i&&Math.abs(e[10]-t[10])<i}function _(e,t){return e[0]===t[0]&&e[1]===t[1]&&e[2]===t[2]&&e[4]===t[4]&&e[5]===t[5]&&e[6]===t[6]&&e[8]===t[8]&&e[9]===t[9]&&e[10]===t[10]}function v(t){let n=t??new e(12);return n[0]=1,n[1]=0,n[2]=0,n[4]=0,n[5]=1,n[6]=0,n[8]=0,n[9]=0,n[10]=1,n}function y(t,n){let r=n??new e(12);if(r===t){let e;return e=t[1],t[1]=t[4],t[4]=e,e=t[2],t[2]=t[8],t[8]=e,e=t[6],t[6]=t[9],t[9]=e,r}let i=t[0],a=t[1],o=t[2],s=t[4],c=t[5],l=t[6],u=t[8],d=t[9],f=t[10];return r[0]=i,r[1]=s,r[2]=u,r[4]=a,r[5]=c,r[6]=d,r[8]=o,r[9]=l,r[10]=f,r}function b(t,n){let r=n??new e(12),i=t[0],a=t[1],o=t[2],s=t[4],c=t[5],l=t[6],u=t[8],d=t[9],f=t[10],p=f*c-l*d,m=-f*s+l*u,h=d*s-c*u,g=1/(i*p+a*m+o*h);return r[0]=p*g,r[1]=(-f*a+o*d)*g,r[2]=(l*a-o*c)*g,r[4]=m*g,r[5]=(f*i-o*u)*g,r[6]=(-l*i+o*s)*g,r[8]=h*g,r[9]=(-d*i+a*u)*g,r[10]=(c*i-a*s)*g,r}function x(e){let t=e[0],n=e[1],r=e[2],i=e[4],a=e[5],o=e[6],s=e[8],c=e[9],l=e[10];return t*(a*l-c*o)-i*(n*l-c*r)+s*(n*o-a*r)}let S=b;function C(t,n,r){let i=r??new e(12),a=t[0],o=t[1],s=t[2],c=t[4],l=t[5],u=t[6],d=t[8],f=t[9],p=t[10],m=n[0],h=n[1],g=n[2],_=n[4],v=n[5],y=n[6],b=n[8],x=n[9],S=n[10];return i[0]=a*m+c*h+d*g,i[1]=o*m+l*h+f*g,i[2]=s*m+u*h+p*g,i[4]=a*_+c*v+d*y,i[5]=o*_+l*v+f*y,i[6]=s*_+u*v+p*y,i[8]=a*b+c*x+d*S,i[9]=o*b+l*x+f*S,i[10]=s*b+u*x+p*S,i}let w=C;function T(e,t,n){let r=n??v();return e!==r&&(r[0]=e[0],r[1]=e[1],r[2]=e[2],r[4]=e[4],r[5]=e[5],r[6]=e[6]),r[8]=t[0],r[9]=t[1],r[10]=1,r}function E(e,n){let r=n??t.create();return r[0]=e[8],r[1]=e[9],r}function D(e,n,r){let i=r??t.create(),a=n*4;return i[0]=e[a+0],i[1]=e[a+1],i}function O(e,t,n,r){let i=r===e?e:m(e,r),a=n*4;return i[a+0]=t[0],i[a+1]=t[1],i}function k(e,n){let r=n??t.create(),i=e[0],a=e[1],o=e[4],s=e[5];return r[0]=Math.sqrt(i*i+a*a),r[1]=Math.sqrt(o*o+s*s),r}function A(e,t){let r=t??n.create(),i=e[0],a=e[1],o=e[2],s=e[4],c=e[5],l=e[6],u=e[8],d=e[9],f=e[10];return r[0]=Math.sqrt(i*i+a*a+o*o),r[1]=Math.sqrt(s*s+c*c+l*l),r[2]=Math.sqrt(u*u+d*d+f*f),r}function j(t,n){let r=n??new e(12);return r[0]=1,r[1]=0,r[2]=0,r[4]=0,r[5]=1,r[6]=0,r[8]=t[0],r[9]=t[1],r[10]=1,r}function M(t,n,r){let i=r??new e(12),a=n[0],o=n[1],s=t[0],c=t[1],l=t[2],u=t[4],d=t[5],f=t[6],p=t[8],m=t[9],h=t[10];return t!==i&&(i[0]=s,i[1]=c,i[2]=l,i[4]=u,i[5]=d,i[6]=f),i[8]=s*a+u*o+p,i[9]=c*a+d*o+m,i[10]=l*a+f*o+h,i}function N(t,n){let r=n??new e(12),i=Math.cos(t),a=Math.sin(t);return r[0]=i,r[1]=a,r[2]=0,r[4]=-a,r[5]=i,r[6]=0,r[8]=0,r[9]=0,r[10]=1,r}function P(t,n,r){let i=r??new e(12),a=t[0],o=t[1],s=t[2],c=t[4],l=t[5],u=t[6],d=Math.cos(n),f=Math.sin(n);return i[0]=d*a+f*c,i[1]=d*o+f*l,i[2]=d*s+f*u,i[4]=d*c-f*a,i[5]=d*l-f*o,i[6]=d*u-f*s,t!==i&&(i[8]=t[8],i[9]=t[9],i[10]=t[10]),i}function F(t,n){let r=n??new e(12),i=Math.cos(t),a=Math.sin(t);return r[0]=1,r[1]=0,r[2]=0,r[4]=0,r[5]=i,r[6]=a,r[8]=0,r[9]=-a,r[10]=i,r}function I(t,n,r){let i=r??new e(12),a=t[4],o=t[5],s=t[6],c=t[8],l=t[9],u=t[10],d=Math.cos(n),f=Math.sin(n);return i[4]=d*a+f*c,i[5]=d*o+f*l,i[6]=d*s+f*u,i[8]=d*c-f*a,i[9]=d*l-f*o,i[10]=d*u-f*s,t!==i&&(i[0]=t[0],i[1]=t[1],i[2]=t[2]),i}function ee(t,n){let r=n??new e(12),i=Math.cos(t),a=Math.sin(t);return r[0]=i,r[1]=0,r[2]=-a,r[4]=0,r[5]=1,r[6]=0,r[8]=a,r[9]=0,r[10]=i,r}function L(t,n,r){let i=r??new e(12),a=t[0],o=t[1],s=t[2],c=t[8],l=t[9],u=t[10],d=Math.cos(n),f=Math.sin(n);return i[0]=d*a-f*c,i[1]=d*o-f*l,i[2]=d*s-f*u,i[8]=d*c+f*a,i[9]=d*l+f*o,i[10]=d*u+f*s,t!==i&&(i[4]=t[4],i[5]=t[5],i[6]=t[6]),i}let R=N,te=P;function ne(t,n){let r=n??new e(12);return r[0]=t[0],r[1]=0,r[2]=0,r[4]=0,r[5]=t[1],r[6]=0,r[8]=0,r[9]=0,r[10]=1,r}function re(t,n,r){let i=r??new e(12),a=n[0],o=n[1];return i[0]=a*t[0],i[1]=a*t[1],i[2]=a*t[2],i[4]=o*t[4],i[5]=o*t[5],i[6]=o*t[6],t!==i&&(i[8]=t[8],i[9]=t[9],i[10]=t[10]),i}function z(t,n){let r=n??new e(12);return r[0]=t[0],r[1]=0,r[2]=0,r[4]=0,r[5]=t[1],r[6]=0,r[8]=0,r[9]=0,r[10]=t[2],r}function B(t,n,r){let i=r??new e(12),a=n[0],o=n[1],s=n[2];return i[0]=a*t[0],i[1]=a*t[1],i[2]=a*t[2],i[4]=o*t[4],i[5]=o*t[5],i[6]=o*t[6],i[8]=s*t[8],i[9]=s*t[9],i[10]=s*t[10],i}function ie(t,n){let r=n??new e(12);return r[0]=t,r[1]=0,r[2]=0,r[4]=0,r[5]=t,r[6]=0,r[8]=0,r[9]=0,r[10]=1,r}function ae(t,n,r){let i=r??new e(12);return i[0]=n*t[0],i[1]=n*t[1],i[2]=n*t[2],i[4]=n*t[4],i[5]=n*t[5],i[6]=n*t[6],t!==i&&(i[8]=t[8],i[9]=t[9],i[10]=t[10]),i}function V(t,n){let r=n??new e(12);return r[0]=t,r[1]=0,r[2]=0,r[4]=0,r[5]=t,r[6]=0,r[8]=0,r[9]=0,r[10]=t,r}function H(t,n,r){let i=r??new e(12);return i[0]=n*t[0],i[1]=n*t[1],i[2]=n*t[2],i[4]=n*t[4],i[5]=n*t[5],i[6]=n*t[6],i[8]=n*t[8],i[9]=n*t[9],i[10]=n*t[10],i}return{add:p,clone:h,copy:m,create:r,determinant:x,equals:_,equalsApproximately:g,fromMat4:o,fromQuat:c,get3DScaling:A,getAxis:D,getScaling:k,getTranslation:E,identity:v,inverse:b,invert:S,mul:w,mulScalar:f,multiply:C,multiplyScalar:d,negate:l,rotate:P,rotateX:I,rotateY:L,rotateZ:te,rotation:N,rotationX:F,rotationY:ee,rotationZ:R,scale:re,scale3D:B,scaling:ne,scaling3D:z,set:a,setAxis:O,setTranslation:T,translate:M,translation:j,transpose:y,uniformScale:ae,uniformScale3D:H,uniformScaling:ie,uniformScaling3D:V}}const f=/* @__PURE__ */ new Map;function p(e){let t=f.get(e);return t||(t=d(e),f.set(e,t)),t}function m(e){let t=u(e);function n(t,n,r,i,a,o,s,c,l,u,d,f,p,m,h,g){let _=new e(16);return t!==void 0&&(_[0]=t,n!==void 0&&(_[1]=n,r!==void 0&&(_[2]=r,i!==void 0&&(_[3]=i,a!==void 0&&(_[4]=a,o!==void 0&&(_[5]=o,s!==void 0&&(_[6]=s,c!==void 0&&(_[7]=c,l!==void 0&&(_[8]=l,u!==void 0&&(_[9]=u,d!==void 0&&(_[10]=d,f!==void 0&&(_[11]=f,p!==void 0&&(_[12]=p,m!==void 0&&(_[13]=m,h!==void 0&&(_[14]=h,g!==void 0&&(_[15]=g)))))))))))))))),_}function r(t,n,r,i,a,o,s,c,l,u,d,f,p,m,h,g,_){let v=_??new e(16);return v[0]=t,v[1]=n,v[2]=r,v[3]=i,v[4]=a,v[5]=o,v[6]=s,v[7]=c,v[8]=l,v[9]=u,v[10]=d,v[11]=f,v[12]=p,v[13]=m,v[14]=h,v[15]=g,v}function a(t,n){let r=n??new e(16);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[3]=0,r[4]=t[4],r[5]=t[5],r[6]=t[6],r[7]=0,r[8]=t[8],r[9]=t[9],r[10]=t[10],r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function o(t,n){let r=n??new e(16),i=t[0],a=t[1],o=t[2],s=t[3],c=i+i,l=a+a,u=o+o,d=i*c,f=a*c,p=a*l,m=o*c,h=o*l,g=o*u,_=s*c,v=s*l,y=s*u;return r[0]=1-p-g,r[1]=f+y,r[2]=m-v,r[3]=0,r[4]=f-y,r[5]=1-d-g,r[6]=h+_,r[7]=0,r[8]=m+v,r[9]=h-_,r[10]=1-d-p,r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function s(t,n){let r=n??new e(16);return r[0]=-t[0],r[1]=-t[1],r[2]=-t[2],r[3]=-t[3],r[4]=-t[4],r[5]=-t[5],r[6]=-t[6],r[7]=-t[7],r[8]=-t[8],r[9]=-t[9],r[10]=-t[10],r[11]=-t[11],r[12]=-t[12],r[13]=-t[13],r[14]=-t[14],r[15]=-t[15],r}function c(t,n,r){let i=r??new e(16);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i[2]=t[2]+n[2],i[3]=t[3]+n[3],i[4]=t[4]+n[4],i[5]=t[5]+n[5],i[6]=t[6]+n[6],i[7]=t[7]+n[7],i[8]=t[8]+n[8],i[9]=t[9]+n[9],i[10]=t[10]+n[10],i[11]=t[11]+n[11],i[12]=t[12]+n[12],i[13]=t[13]+n[13],i[14]=t[14]+n[14],i[15]=t[15]+n[15],i}function l(t,n,r){let i=r??new e(16);return i[0]=t[0]*n,i[1]=t[1]*n,i[2]=t[2]*n,i[3]=t[3]*n,i[4]=t[4]*n,i[5]=t[5]*n,i[6]=t[6]*n,i[7]=t[7]*n,i[8]=t[8]*n,i[9]=t[9]*n,i[10]=t[10]*n,i[11]=t[11]*n,i[12]=t[12]*n,i[13]=t[13]*n,i[14]=t[14]*n,i[15]=t[15]*n,i}let d=l;function f(t,n){let r=n??new e(16);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[3]=t[3],r[4]=t[4],r[5]=t[5],r[6]=t[6],r[7]=t[7],r[8]=t[8],r[9]=t[9],r[10]=t[10],r[11]=t[11],r[12]=t[12],r[13]=t[13],r[14]=t[14],r[15]=t[15],r}let p=f;function m(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i&&Math.abs(e[2]-t[2])<i&&Math.abs(e[3]-t[3])<i&&Math.abs(e[4]-t[4])<i&&Math.abs(e[5]-t[5])<i&&Math.abs(e[6]-t[6])<i&&Math.abs(e[7]-t[7])<i&&Math.abs(e[8]-t[8])<i&&Math.abs(e[9]-t[9])<i&&Math.abs(e[10]-t[10])<i&&Math.abs(e[11]-t[11])<i&&Math.abs(e[12]-t[12])<i&&Math.abs(e[13]-t[13])<i&&Math.abs(e[14]-t[14])<i&&Math.abs(e[15]-t[15])<i}function h(e,t){return e[0]===t[0]&&e[1]===t[1]&&e[2]===t[2]&&e[3]===t[3]&&e[4]===t[4]&&e[5]===t[5]&&e[6]===t[6]&&e[7]===t[7]&&e[8]===t[8]&&e[9]===t[9]&&e[10]===t[10]&&e[11]===t[11]&&e[12]===t[12]&&e[13]===t[13]&&e[14]===t[14]&&e[15]===t[15]}function g(t){let n=t??new e(16);return n[0]=1,n[1]=0,n[2]=0,n[3]=0,n[4]=0,n[5]=1,n[6]=0,n[7]=0,n[8]=0,n[9]=0,n[10]=1,n[11]=0,n[12]=0,n[13]=0,n[14]=0,n[15]=1,n}function _(t,n){let r=n??new e(16);if(r===t){let e;return e=t[1],t[1]=t[4],t[4]=e,e=t[2],t[2]=t[8],t[8]=e,e=t[3],t[3]=t[12],t[12]=e,e=t[6],t[6]=t[9],t[9]=e,e=t[7],t[7]=t[13],t[13]=e,e=t[11],t[11]=t[14],t[14]=e,r}let i=t[0],a=t[1],o=t[2],s=t[3],c=t[4],l=t[5],u=t[6],d=t[7],f=t[8],p=t[9],m=t[10],h=t[11],g=t[12],_=t[13],v=t[14],y=t[15];return r[0]=i,r[1]=c,r[2]=f,r[3]=g,r[4]=a,r[5]=l,r[6]=p,r[7]=_,r[8]=o,r[9]=u,r[10]=m,r[11]=v,r[12]=s,r[13]=d,r[14]=h,r[15]=y,r}function v(t,n){let r=n??new e(16),i=t[0],a=t[1],o=t[2],s=t[3],c=t[4],l=t[5],u=t[6],d=t[7],f=t[8],p=t[9],m=t[10],h=t[11],g=t[12],_=t[13],v=t[14],y=t[15],b=m*y,x=v*h,S=u*y,C=v*d,w=u*h,T=m*d,E=o*y,D=v*s,O=o*h,k=m*s,A=o*d,j=u*s,M=f*_,N=g*p,P=c*_,F=g*l,I=c*p,ee=f*l,L=i*_,R=g*a,te=i*p,ne=f*a,re=i*l,z=c*a,B=b*l+C*p+w*_-(x*l+S*p+T*_),ie=x*a+E*p+k*_-(b*a+D*p+O*_),ae=S*a+D*l+A*_-(C*a+E*l+j*_),V=T*a+O*l+j*p-(w*a+k*l+A*p),H=1/(i*B+c*ie+f*ae+g*V);return r[0]=H*B,r[1]=H*ie,r[2]=H*ae,r[3]=H*V,r[4]=H*(x*c+S*f+T*g-(b*c+C*f+w*g)),r[5]=H*(b*i+D*f+O*g-(x*i+E*f+k*g)),r[6]=H*(C*i+E*c+j*g-(S*i+D*c+A*g)),r[7]=H*(w*i+k*c+A*f-(T*i+O*c+j*f)),r[8]=H*(M*d+F*h+I*y-(N*d+P*h+ee*y)),r[9]=H*(N*s+L*h+ne*y-(M*s+R*h+te*y)),r[10]=H*(P*s+R*d+re*y-(F*s+L*d+z*y)),r[11]=H*(ee*s+te*d+z*h-(I*s+ne*d+re*h)),r[12]=H*(P*m+ee*v+N*u-(I*v+M*u+F*m)),r[13]=H*(te*v+M*o+R*m-(L*m+ne*v+N*o)),r[14]=H*(L*u+z*v+F*o-(re*v+P*o+R*u)),r[15]=H*(re*m+I*o+ne*u-(te*u+z*m+ee*o)),r}function y(e){let t=e[0],n=e[1],r=e[2],i=e[3],a=e[4],o=e[5],s=e[6],c=e[7],l=e[8],u=e[9],d=e[10],f=e[11],p=e[12],m=e[13],h=e[14],g=e[15],_=d*g,v=h*f,y=s*g,b=h*c,x=s*f,S=d*c,C=r*g,w=h*i,T=r*f,E=d*i,D=r*c,O=s*i,k=_*o+b*u+x*m-(v*o+y*u+S*m),A=v*n+C*u+E*m-(_*n+w*u+T*m),j=y*n+w*o+D*m-(b*n+C*o+O*m),M=S*n+T*o+O*u-(x*n+E*o+D*u);return t*k+a*A+l*j+p*M}let b=v;function x(t,n,r){let i=r??new e(16),a=t[0],o=t[1],s=t[2],c=t[3],l=t[4],u=t[5],d=t[6],f=t[7],p=t[8],m=t[9],h=t[10],g=t[11],_=t[12],v=t[13],y=t[14],b=t[15],x=n[0],S=n[1],C=n[2],w=n[3],T=n[4],E=n[5],D=n[6],O=n[7],k=n[8],A=n[9],j=n[10],M=n[11],N=n[12],P=n[13],F=n[14],I=n[15];return i[0]=a*x+l*S+p*C+_*w,i[1]=o*x+u*S+m*C+v*w,i[2]=s*x+d*S+h*C+y*w,i[3]=c*x+f*S+g*C+b*w,i[4]=a*T+l*E+p*D+_*O,i[5]=o*T+u*E+m*D+v*O,i[6]=s*T+d*E+h*D+y*O,i[7]=c*T+f*E+g*D+b*O,i[8]=a*k+l*A+p*j+_*M,i[9]=o*k+u*A+m*j+v*M,i[10]=s*k+d*A+h*j+y*M,i[11]=c*k+f*A+g*j+b*M,i[12]=a*N+l*P+p*F+_*I,i[13]=o*N+u*P+m*F+v*I,i[14]=s*N+d*P+h*F+y*I,i[15]=c*N+f*P+g*F+b*I,i}let S=x;function C(e,t,n){let r=n??g();return e!==r&&(r[0]=e[0],r[1]=e[1],r[2]=e[2],r[3]=e[3],r[4]=e[4],r[5]=e[5],r[6]=e[6],r[7]=e[7],r[8]=e[8],r[9]=e[9],r[10]=e[10],r[11]=e[11]),r[12]=t[0],r[13]=t[1],r[14]=t[2],r[15]=1,r}function w(e,n){let r=n??t.create();return r[0]=e[12],r[1]=e[13],r[2]=e[14],r}function T(e,n,r){let i=r??t.create(),a=n*4;return i[0]=e[a+0],i[1]=e[a+1],i[2]=e[a+2],i}function E(e,t,n,r){let i=r===e?r:f(e,r),a=n*4;return i[a+0]=t[0],i[a+1]=t[1],i[a+2]=t[2],i}function D(e,n){let r=n??t.create(),i=e[0],a=e[1],o=e[2],s=e[4],c=e[5],l=e[6],u=e[8],d=e[9],f=e[10];return r[0]=Math.sqrt(i*i+a*a+o*o),r[1]=Math.sqrt(s*s+c*c+l*l),r[2]=Math.sqrt(u*u+d*d+f*f),r}function O(t,n,r,i,a){let o=a??new e(16),s=Math.tan(Math.PI*.5-.5*t);if(o[0]=s/n,o[1]=0,o[2]=0,o[3]=0,o[4]=0,o[5]=s,o[6]=0,o[7]=0,o[8]=0,o[9]=0,o[11]=-1,o[12]=0,o[13]=0,o[15]=0,Number.isFinite(i)){let e=1/(r-i);o[10]=i*e,o[14]=i*r*e}else o[10]=-1,o[14]=-r;return o}function k(t,n,r,i=1/0,a){let o=a??new e(16),s=1/Math.tan(t*.5);if(o[0]=s/n,o[1]=0,o[2]=0,o[3]=0,o[4]=0,o[5]=s,o[6]=0,o[7]=0,o[8]=0,o[9]=0,o[11]=-1,o[12]=0,o[13]=0,o[15]=0,i===1/0)o[10]=0,o[14]=r;else{let e=1/(i-r);o[10]=r*e,o[14]=i*r*e}return o}function A(t,n,r,i,a,o,s){let c=s??new e(16);return c[0]=2/(n-t),c[1]=0,c[2]=0,c[3]=0,c[4]=0,c[5]=2/(i-r),c[6]=0,c[7]=0,c[8]=0,c[9]=0,c[10]=1/(a-o),c[11]=0,c[12]=(n+t)/(t-n),c[13]=(i+r)/(r-i),c[14]=a/(a-o),c[15]=1,c}function j(t,n,r,i,a,o,s){let c=s??new e(16),l=n-t,u=i-r,d=a-o;return c[0]=2*a/l,c[1]=0,c[2]=0,c[3]=0,c[4]=0,c[5]=2*a/u,c[6]=0,c[7]=0,c[8]=(t+n)/l,c[9]=(i+r)/u,c[10]=o/d,c[11]=-1,c[12]=0,c[13]=0,c[14]=a*o/d,c[15]=0,c}function M(t,n,r,i,a,o=1/0,s){let c=s??new e(16),l=n-t,u=i-r;if(c[0]=2*a/l,c[1]=0,c[2]=0,c[3]=0,c[4]=0,c[5]=2*a/u,c[6]=0,c[7]=0,c[8]=(t+n)/l,c[9]=(i+r)/u,c[11]=-1,c[12]=0,c[13]=0,c[15]=0,o===1/0)c[10]=0,c[14]=a;else{let e=1/(o-a);c[10]=a*e,c[14]=o*a*e}return c}let N=t.create(),P=t.create(),F=t.create();function I(n,r,i,a){let o=a??new e(16);return t.normalize(t.subtract(r,n,F),F),t.normalize(t.cross(i,F,N),N),t.normalize(t.cross(F,N,P),P),o[0]=N[0],o[1]=N[1],o[2]=N[2],o[3]=0,o[4]=P[0],o[5]=P[1],o[6]=P[2],o[7]=0,o[8]=F[0],o[9]=F[1],o[10]=F[2],o[11]=0,o[12]=n[0],o[13]=n[1],o[14]=n[2],o[15]=1,o}function ee(n,r,i,a){let o=a??new e(16);return t.normalize(t.subtract(n,r,F),F),t.normalize(t.cross(i,F,N),N),t.normalize(t.cross(F,N,P),P),o[0]=N[0],o[1]=N[1],o[2]=N[2],o[3]=0,o[4]=P[0],o[5]=P[1],o[6]=P[2],o[7]=0,o[8]=F[0],o[9]=F[1],o[10]=F[2],o[11]=0,o[12]=n[0],o[13]=n[1],o[14]=n[2],o[15]=1,o}function L(n,r,i,a){let o=a??new e(16);return t.normalize(t.subtract(n,r,F),F),t.normalize(t.cross(i,F,N),N),t.normalize(t.cross(F,N,P),P),o[0]=N[0],o[1]=P[0],o[2]=F[0],o[3]=0,o[4]=N[1],o[5]=P[1],o[6]=F[1],o[7]=0,o[8]=N[2],o[9]=P[2],o[10]=F[2],o[11]=0,o[12]=-(N[0]*n[0]+N[1]*n[1]+N[2]*n[2]),o[13]=-(P[0]*n[0]+P[1]*n[1]+P[2]*n[2]),o[14]=-(F[0]*n[0]+F[1]*n[1]+F[2]*n[2]),o[15]=1,o}function R(t,n){let r=n??new e(16);return r[0]=1,r[1]=0,r[2]=0,r[3]=0,r[4]=0,r[5]=1,r[6]=0,r[7]=0,r[8]=0,r[9]=0,r[10]=1,r[11]=0,r[12]=t[0],r[13]=t[1],r[14]=t[2],r[15]=1,r}function te(t,n,r){let i=r??new e(16),a=n[0],o=n[1],s=n[2],c=t[0],l=t[1],u=t[2],d=t[3],f=t[4],p=t[5],m=t[6],h=t[7],g=t[8],_=t[9],v=t[10],y=t[11],b=t[12],x=t[13],S=t[14],C=t[15];return t!==i&&(i[0]=c,i[1]=l,i[2]=u,i[3]=d,i[4]=f,i[5]=p,i[6]=m,i[7]=h,i[8]=g,i[9]=_,i[10]=v,i[11]=y),i[12]=c*a+f*o+g*s+b,i[13]=l*a+p*o+_*s+x,i[14]=u*a+m*o+v*s+S,i[15]=d*a+h*o+y*s+C,i}function ne(t,n){let r=n??new e(16),i=Math.cos(t),a=Math.sin(t);return r[0]=1,r[1]=0,r[2]=0,r[3]=0,r[4]=0,r[5]=i,r[6]=a,r[7]=0,r[8]=0,r[9]=-a,r[10]=i,r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function re(t,n,r){let i=r??new e(16),a=t[4],o=t[5],s=t[6],c=t[7],l=t[8],u=t[9],d=t[10],f=t[11],p=Math.cos(n),m=Math.sin(n);return i[4]=p*a+m*l,i[5]=p*o+m*u,i[6]=p*s+m*d,i[7]=p*c+m*f,i[8]=p*l-m*a,i[9]=p*u-m*o,i[10]=p*d-m*s,i[11]=p*f-m*c,t!==i&&(i[0]=t[0],i[1]=t[1],i[2]=t[2],i[3]=t[3],i[12]=t[12],i[13]=t[13],i[14]=t[14],i[15]=t[15]),i}function z(t,n){let r=n??new e(16),i=Math.cos(t),a=Math.sin(t);return r[0]=i,r[1]=0,r[2]=-a,r[3]=0,r[4]=0,r[5]=1,r[6]=0,r[7]=0,r[8]=a,r[9]=0,r[10]=i,r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function B(t,n,r){let i=r??new e(16),a=t[0],o=t[1],s=t[2],c=t[3],l=t[8],u=t[9],d=t[10],f=t[11],p=Math.cos(n),m=Math.sin(n);return i[0]=p*a-m*l,i[1]=p*o-m*u,i[2]=p*s-m*d,i[3]=p*c-m*f,i[8]=p*l+m*a,i[9]=p*u+m*o,i[10]=p*d+m*s,i[11]=p*f+m*c,t!==i&&(i[4]=t[4],i[5]=t[5],i[6]=t[6],i[7]=t[7],i[12]=t[12],i[13]=t[13],i[14]=t[14],i[15]=t[15]),i}function ie(t,n){let r=n??new e(16),i=Math.cos(t),a=Math.sin(t);return r[0]=i,r[1]=a,r[2]=0,r[3]=0,r[4]=-a,r[5]=i,r[6]=0,r[7]=0,r[8]=0,r[9]=0,r[10]=1,r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function ae(t,n,r){let i=r??new e(16),a=t[0],o=t[1],s=t[2],c=t[3],l=t[4],u=t[5],d=t[6],f=t[7],p=Math.cos(n),m=Math.sin(n);return i[0]=p*a+m*l,i[1]=p*o+m*u,i[2]=p*s+m*d,i[3]=p*c+m*f,i[4]=p*l-m*a,i[5]=p*u-m*o,i[6]=p*d-m*s,i[7]=p*f-m*c,t!==i&&(i[8]=t[8],i[9]=t[9],i[10]=t[10],i[11]=t[11],i[12]=t[12],i[13]=t[13],i[14]=t[14],i[15]=t[15]),i}function V(t,n,r){let i=r??new e(16),a=t[0],o=t[1],s=t[2],c=Math.sqrt(a*a+o*o+s*s);a/=c,o/=c,s/=c;let l=a*a,u=o*o,d=s*s,f=Math.cos(n),p=Math.sin(n),m=1-f;return i[0]=l+(1-l)*f,i[1]=a*o*m+s*p,i[2]=a*s*m-o*p,i[3]=0,i[4]=a*o*m-s*p,i[5]=u+(1-u)*f,i[6]=o*s*m+a*p,i[7]=0,i[8]=a*s*m+o*p,i[9]=o*s*m-a*p,i[10]=d+(1-d)*f,i[11]=0,i[12]=0,i[13]=0,i[14]=0,i[15]=1,i}let H=V;function oe(t,n,r,i){let a=i??new e(16),o=n[0],s=n[1],c=n[2],l=Math.sqrt(o*o+s*s+c*c);o/=l,s/=l,c/=l;let u=o*o,d=s*s,f=c*c,p=Math.cos(r),m=Math.sin(r),h=1-p,g=u+(1-u)*p,_=o*s*h+c*m,v=o*c*h-s*m,y=o*s*h-c*m,b=d+(1-d)*p,x=s*c*h+o*m,S=o*c*h+s*m,C=s*c*h-o*m,w=f+(1-f)*p,T=t[0],E=t[1],D=t[2],O=t[3],k=t[4],A=t[5],j=t[6],M=t[7],N=t[8],P=t[9],F=t[10],I=t[11];return a[0]=g*T+_*k+v*N,a[1]=g*E+_*A+v*P,a[2]=g*D+_*j+v*F,a[3]=g*O+_*M+v*I,a[4]=y*T+b*k+x*N,a[5]=y*E+b*A+x*P,a[6]=y*D+b*j+x*F,a[7]=y*O+b*M+x*I,a[8]=S*T+C*k+w*N,a[9]=S*E+C*A+w*P,a[10]=S*D+C*j+w*F,a[11]=S*O+C*M+w*I,t!==a&&(a[12]=t[12],a[13]=t[13],a[14]=t[14],a[15]=t[15]),a}let se=oe;function ce(t,n){let r=n??new e(16);return r[0]=t[0],r[1]=0,r[2]=0,r[3]=0,r[4]=0,r[5]=t[1],r[6]=0,r[7]=0,r[8]=0,r[9]=0,r[10]=t[2],r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function le(t,n,r){let i=r??new e(16),a=n[0],o=n[1],s=n[2];return i[0]=a*t[0],i[1]=a*t[1],i[2]=a*t[2],i[3]=a*t[3],i[4]=o*t[4],i[5]=o*t[5],i[6]=o*t[6],i[7]=o*t[7],i[8]=s*t[8],i[9]=s*t[9],i[10]=s*t[10],i[11]=s*t[11],t!==i&&(i[12]=t[12],i[13]=t[13],i[14]=t[14],i[15]=t[15]),i}function ue(t,n){let r=n??new e(16);return r[0]=t,r[1]=0,r[2]=0,r[3]=0,r[4]=0,r[5]=t,r[6]=0,r[7]=0,r[8]=0,r[9]=0,r[10]=t,r[11]=0,r[12]=0,r[13]=0,r[14]=0,r[15]=1,r}function U(t,n,r){let i=r??new e(16);return i[0]=n*t[0],i[1]=n*t[1],i[2]=n*t[2],i[3]=n*t[3],i[4]=n*t[4],i[5]=n*t[5],i[6]=n*t[6],i[7]=n*t[7],i[8]=n*t[8],i[9]=n*t[9],i[10]=n*t[10],i[11]=n*t[11],t!==i&&(i[12]=t[12],i[13]=t[13],i[14]=t[14],i[15]=t[15]),i}return{add:c,aim:I,axisRotate:oe,axisRotation:V,cameraAim:ee,clone:p,copy:f,create:n,determinant:y,equals:h,equalsApproximately:m,fromMat3:a,fromQuat:o,frustum:j,frustumReverseZ:M,getAxis:T,getScaling:D,getTranslation:w,identity:g,inverse:v,invert:b,lookAt:L,mul:S,mulScalar:d,multiply:x,multiplyScalar:l,negate:s,ortho:A,perspective:O,perspectiveReverseZ:k,rotate:se,rotateX:re,rotateY:B,rotateZ:ae,rotation:H,rotationX:ne,rotationY:z,rotationZ:ie,scale:le,scaling:ce,set:r,setAxis:E,setTranslation:C,translate:te,translation:R,transpose:_,uniformScale:U,uniformScaling:ue}}const h=/* @__PURE__ */ new Map;function g(e){let t=h.get(e);return t||(t=m(e),h.set(e,t)),t}function _(e){let t=u(e);function n(t,n,r,i){let a=new e(4);return t!==void 0&&(a[0]=t,n!==void 0&&(a[1]=n,r!==void 0&&(a[2]=r,i!==void 0&&(a[3]=i)))),a}let r=n;function a(t,n,r,i,a){let o=a??new e(4);return o[0]=t,o[1]=n,o[2]=r,o[3]=i,o}function o(t,n,r){let i=r??new e(4),a=n*.5,o=Math.sin(a);return i[0]=o*t[0],i[1]=o*t[1],i[2]=o*t[2],i[3]=Math.cos(a),i}function s(e,n){let r=n??t.create(3),a=Math.acos(e[3])*2,o=Math.sin(a*.5);return o>i?(r[0]=e[0]/o,r[1]=e[1]/o,r[2]=e[2]/o):(r[0]=1,r[1]=0,r[2]=0),{angle:a,axis:r}}function c(e,t){let n=O(e,t);return Math.acos(2*n*n-1)}function l(t,n,r){let i=r??new e(4),a=t[0],o=t[1],s=t[2],c=t[3],l=n[0],u=n[1],d=n[2],f=n[3];return i[0]=a*f+c*l+o*d-s*u,i[1]=o*f+c*u+s*l-a*d,i[2]=s*f+c*d+a*u-o*l,i[3]=c*f-a*l-o*u-s*d,i}let d=l;function f(t,n,r){let i=r??new e(4),a=n*.5,o=t[0],s=t[1],c=t[2],l=t[3],u=Math.sin(a),d=Math.cos(a);return i[0]=o*d+l*u,i[1]=s*d+c*u,i[2]=c*d-s*u,i[3]=l*d-o*u,i}function p(t,n,r){let i=r??new e(4),a=n*.5,o=t[0],s=t[1],c=t[2],l=t[3],u=Math.sin(a),d=Math.cos(a);return i[0]=o*d-c*u,i[1]=s*d+l*u,i[2]=c*d+o*u,i[3]=l*d-s*u,i}function m(t,n,r){let i=r??new e(4),a=n*.5,o=t[0],s=t[1],c=t[2],l=t[3],u=Math.sin(a),d=Math.cos(a);return i[0]=o*d+s*u,i[1]=s*d-o*u,i[2]=c*d+l*u,i[3]=l*d-c*u,i}function h(t,n,r,a){let o=a??new e(4),s=t[0],c=t[1],l=t[2],u=t[3],d=n[0],f=n[1],p=n[2],m=n[3],h=s*d+c*f+l*p+u*m;h<0&&(h=-h,d=-d,f=-f,p=-p,m=-m);let g,_;if(1-h>i){let e=Math.acos(h),t=Math.sin(e);g=Math.sin((1-r)*e)/t,_=Math.sin(r*e)/t}else g=1-r,_=r;return o[0]=g*s+_*d,o[1]=g*c+_*f,o[2]=g*l+_*p,o[3]=g*u+_*m,o}function g(t,n){let r=n??new e(4),i=t[0],a=t[1],o=t[2],s=t[3],c=i*i+a*a+o*o+s*s,l=c?1/c:0;return r[0]=-i*l,r[1]=-a*l,r[2]=-o*l,r[3]=s*l,r}function _(t,n){let r=n??new e(4);return r[0]=-t[0],r[1]=-t[1],r[2]=-t[2],r[3]=t[3],r}function v(t,n){let r=n??new e(4),i=t[0]+t[5]+t[10];if(i>0){let e=Math.sqrt(i+1);r[3]=.5*e;let n=.5/e;r[0]=(t[6]-t[9])*n,r[1]=(t[8]-t[2])*n,r[2]=(t[1]-t[4])*n}else{let e=0;t[5]>t[0]&&(e=1),t[10]>t[e*4+e]&&(e=2);let n=(e+1)%3,i=(e+2)%3,a=Math.sqrt(t[e*4+e]-t[n*4+n]-t[i*4+i]+1);r[e]=.5*a;let o=.5/a;r[3]=(t[n*4+i]-t[i*4+n])*o,r[n]=(t[n*4+e]+t[e*4+n])*o,r[i]=(t[i*4+e]+t[e*4+i])*o}return r}function y(t,n,r,i,a){let o=a??new e(4),s=t*.5,c=n*.5,l=r*.5,u=Math.sin(s),d=Math.cos(s),f=Math.sin(c),p=Math.cos(c),m=Math.sin(l),h=Math.cos(l);switch(i){case`xyz`:o[0]=u*p*h+d*f*m,o[1]=d*f*h-u*p*m,o[2]=d*p*m+u*f*h,o[3]=d*p*h-u*f*m;break;case`xzy`:o[0]=u*p*h-d*f*m,o[1]=d*f*h-u*p*m,o[2]=d*p*m+u*f*h,o[3]=d*p*h+u*f*m;break;case`yxz`:o[0]=u*p*h+d*f*m,o[1]=d*f*h-u*p*m,o[2]=d*p*m-u*f*h,o[3]=d*p*h+u*f*m;break;case`yzx`:o[0]=u*p*h+d*f*m,o[1]=d*f*h+u*p*m,o[2]=d*p*m-u*f*h,o[3]=d*p*h-u*f*m;break;case`zxy`:o[0]=u*p*h-d*f*m,o[1]=d*f*h+u*p*m,o[2]=d*p*m+u*f*h,o[3]=d*p*h-u*f*m;break;case`zyx`:o[0]=u*p*h-d*f*m,o[1]=d*f*h+u*p*m,o[2]=d*p*m-u*f*h,o[3]=d*p*h+u*f*m;break;default:throw Error(`Unknown rotation order: ${i}`)}return o}function b(t,n){let r=n??new e(4);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[3]=t[3],r}let x=b;function S(t,n,r){let i=r??new e(4);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i[2]=t[2]+n[2],i[3]=t[3]+n[3],i}function C(t,n,r){let i=r??new e(4);return i[0]=t[0]-n[0],i[1]=t[1]-n[1],i[2]=t[2]-n[2],i[3]=t[3]-n[3],i}let w=C;function T(t,n,r){let i=r??new e(4);return i[0]=t[0]*n,i[1]=t[1]*n,i[2]=t[2]*n,i[3]=t[3]*n,i}let E=T;function D(t,n,r){let i=r??new e(4);return i[0]=t[0]/n,i[1]=t[1]/n,i[2]=t[2]/n,i[3]=t[3]/n,i}function O(e,t){return e[0]*t[0]+e[1]*t[1]+e[2]*t[2]+e[3]*t[3]}function k(t,n,r,i){let a=i??new e(4);return a[0]=t[0]+r*(n[0]-t[0]),a[1]=t[1]+r*(n[1]-t[1]),a[2]=t[2]+r*(n[2]-t[2]),a[3]=t[3]+r*(n[3]-t[3]),a}function A(e){let t=e[0],n=e[1],r=e[2],i=e[3];return Math.sqrt(t*t+n*n+r*r+i*i)}let j=A;function M(e){let t=e[0],n=e[1],r=e[2],i=e[3];return t*t+n*n+r*r+i*i}let N=M;function P(t,n){let r=n??new e(4),i=t[0],a=t[1],o=t[2],s=t[3],c=Math.sqrt(i*i+a*a+o*o+s*s);return c>1e-5?(r[0]=i/c,r[1]=a/c,r[2]=o/c,r[3]=s/c):(r[0]=0,r[1]=0,r[2]=0,r[3]=1),r}function F(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i&&Math.abs(e[2]-t[2])<i&&Math.abs(e[3]-t[3])<i}function I(e,t){return e[0]===t[0]&&e[1]===t[1]&&e[2]===t[2]&&e[3]===t[3]}function ee(t){let n=t??new e(4);return n[0]=0,n[1]=0,n[2]=0,n[3]=1,n}let L=t.create(),R=t.create(),te=t.create();function ne(n,r,i){let a=i??new e(4),s=t.dot(n,r);return s<-.999999?(t.cross(R,n,L),t.len(L)<1e-6&&t.cross(te,n,L),t.normalize(L,L),o(L,Math.PI,a),a):s>.999999?(a[0]=0,a[1]=0,a[2]=0,a[3]=1,a):(t.cross(n,r,L),a[0]=L[0],a[1]=L[1],a[2]=L[2],a[3]=1+s,P(a,a))}let re=new e(4),z=new e(4);function B(t,n,r,i,a,o){let s=o??new e(4);return h(t,i,a,re),h(n,r,a,z),h(re,z,2*a*(1-a),s),s}return{create:n,fromValues:r,set:a,fromAxisAngle:o,toAxisAngle:s,angle:c,multiply:l,mul:d,rotateX:f,rotateY:p,rotateZ:m,slerp:h,inverse:g,conjugate:_,fromMat:v,fromEuler:y,copy:b,clone:x,add:S,subtract:C,sub:w,mulScalar:T,scale:E,divScalar:D,dot:O,lerp:k,length:A,len:j,lengthSq:M,lenSq:N,normalize:P,equalsApproximately:F,equals:I,identity:ee,rotationTo:ne,sqlerp:B}}const v=/* @__PURE__ */ new Map;function y(e){let t=v.get(e);return t||(t=_(e),v.set(e,t)),t}function b(e){function t(t,n,r,i){let a=new e(4);return t!==void 0&&(a[0]=t,n!==void 0&&(a[1]=n,r!==void 0&&(a[2]=r,i!==void 0&&(a[3]=i)))),a}let n=t;function r(t,n,r,i,a){let o=a??new e(4);return o[0]=t,o[1]=n,o[2]=r,o[3]=i,o}function a(t,n){let r=n??new e(4);return r[0]=Math.ceil(t[0]),r[1]=Math.ceil(t[1]),r[2]=Math.ceil(t[2]),r[3]=Math.ceil(t[3]),r}function o(t,n){let r=n??new e(4);return r[0]=Math.floor(t[0]),r[1]=Math.floor(t[1]),r[2]=Math.floor(t[2]),r[3]=Math.floor(t[3]),r}function s(t,n){let r=n??new e(4);return r[0]=Math.round(t[0]),r[1]=Math.round(t[1]),r[2]=Math.round(t[2]),r[3]=Math.round(t[3]),r}function c(t,n=0,r=1,i){let a=i??new e(4);return a[0]=Math.min(r,Math.max(n,t[0])),a[1]=Math.min(r,Math.max(n,t[1])),a[2]=Math.min(r,Math.max(n,t[2])),a[3]=Math.min(r,Math.max(n,t[3])),a}function l(t,n,r){let i=r??new e(4);return i[0]=t[0]+n[0],i[1]=t[1]+n[1],i[2]=t[2]+n[2],i[3]=t[3]+n[3],i}function u(t,n,r,i){let a=i??new e(4);return a[0]=t[0]+n[0]*r,a[1]=t[1]+n[1]*r,a[2]=t[2]+n[2]*r,a[3]=t[3]+n[3]*r,a}function d(t,n,r){let i=r??new e(4);return i[0]=t[0]-n[0],i[1]=t[1]-n[1],i[2]=t[2]-n[2],i[3]=t[3]-n[3],i}let f=d;function p(e,t){return Math.abs(e[0]-t[0])<i&&Math.abs(e[1]-t[1])<i&&Math.abs(e[2]-t[2])<i&&Math.abs(e[3]-t[3])<i}function m(e,t){return e[0]===t[0]&&e[1]===t[1]&&e[2]===t[2]&&e[3]===t[3]}function h(t,n,r,i){let a=i??new e(4);return a[0]=t[0]+r*(n[0]-t[0]),a[1]=t[1]+r*(n[1]-t[1]),a[2]=t[2]+r*(n[2]-t[2]),a[3]=t[3]+r*(n[3]-t[3]),a}function g(t,n,r,i){let a=i??new e(4);return a[0]=t[0]+r[0]*(n[0]-t[0]),a[1]=t[1]+r[1]*(n[1]-t[1]),a[2]=t[2]+r[2]*(n[2]-t[2]),a[3]=t[3]+r[3]*(n[3]-t[3]),a}function _(t,n,r){let i=r??new e(4);return i[0]=Math.max(t[0],n[0]),i[1]=Math.max(t[1],n[1]),i[2]=Math.max(t[2],n[2]),i[3]=Math.max(t[3],n[3]),i}function v(t,n,r){let i=r??new e(4);return i[0]=Math.min(t[0],n[0]),i[1]=Math.min(t[1],n[1]),i[2]=Math.min(t[2],n[2]),i[3]=Math.min(t[3],n[3]),i}function y(t,n,r){let i=r??new e(4);return i[0]=t[0]*n,i[1]=t[1]*n,i[2]=t[2]*n,i[3]=t[3]*n,i}let b=y;function x(t,n,r){let i=r??new e(4);return i[0]=t[0]/n,i[1]=t[1]/n,i[2]=t[2]/n,i[3]=t[3]/n,i}function S(t,n){let r=n??new e(4);return r[0]=1/t[0],r[1]=1/t[1],r[2]=1/t[2],r[3]=1/t[3],r}let C=S;function w(e,t){return e[0]*t[0]+e[1]*t[1]+e[2]*t[2]+e[3]*t[3]}function T(e){let t=e[0],n=e[1],r=e[2],i=e[3];return Math.sqrt(t*t+n*n+r*r+i*i)}let E=T;function D(e){let t=e[0],n=e[1],r=e[2],i=e[3];return t*t+n*n+r*r+i*i}let O=D;function k(e,t){let n=e[0]-t[0],r=e[1]-t[1],i=e[2]-t[2],a=e[3]-t[3];return Math.sqrt(n*n+r*r+i*i+a*a)}let A=k;function j(e,t){let n=e[0]-t[0],r=e[1]-t[1],i=e[2]-t[2],a=e[3]-t[3];return n*n+r*r+i*i+a*a}let M=j;function N(t,n){let r=n??new e(4),i=t[0],a=t[1],o=t[2],s=t[3],c=i*i+a*a+o*o+s*s,l=c>0?1/Math.sqrt(c):1;return r[0]=i*l,r[1]=a*l,r[2]=o*l,r[3]=s*l,r}function P(t,n){let r=n??new e(4);return r[0]=-t[0],r[1]=-t[1],r[2]=-t[2],r[3]=-t[3],r}function F(t,n){let r=n??new e(4);return r[0]=t[0],r[1]=t[1],r[2]=t[2],r[3]=t[3],r}let I=F;function ee(t,n,r){let i=r??new e(4);return i[0]=t[0]*n[0],i[1]=t[1]*n[1],i[2]=t[2]*n[2],i[3]=t[3]*n[3],i}let L=ee;function R(t,n,r){let i=r??new e(4);return i[0]=t[0]/n[0],i[1]=t[1]/n[1],i[2]=t[2]/n[2],i[3]=t[3]/n[3],i}let te=R;function ne(t){let n=t??new e(4);return n[0]=0,n[1]=0,n[2]=0,n[3]=0,n}function re(t,n,r){let i=r??new e(4),a=t[0],o=t[1],s=t[2],c=t[3];return i[0]=n[0]*a+n[4]*o+n[8]*s+n[12]*c,i[1]=n[1]*a+n[5]*o+n[9]*s+n[13]*c,i[2]=n[2]*a+n[6]*o+n[10]*s+n[14]*c,i[3]=n[3]*a+n[7]*o+n[11]*s+n[15]*c,i}function z(t,n,r){let i=r??new e(4);return N(t,i),y(i,n,i)}function B(t,n,r){let i=r??new e(4);return T(t)>n?z(t,n,i):F(t,i)}function ie(t,n,r){return h(t,n,.5,r??new e(4))}return{create:t,fromValues:n,set:r,ceil:a,floor:o,round:s,clamp:c,add:l,addScaled:u,subtract:d,sub:f,equalsApproximately:p,equals:m,lerp:h,lerpV:g,max:_,min:v,mulScalar:y,scale:b,divScalar:x,inverse:S,invert:C,dot:w,length:T,len:E,lengthSq:D,lenSq:O,distance:k,dist:A,distanceSq:j,distSq:M,normalize:N,negate:P,copy:F,clone:I,multiply:ee,mul:L,divide:R,div:te,zero:ne,transformMat4:re,setLength:z,truncate:B,midpoint:ie}}const x=/* @__PURE__ */ new Map;function S(e){let t=x.get(e);return t||(t=b(e),x.set(e,t)),t}function C(e,t,n,r,i,a){return{mat3:p(e),mat4:g(t),quat:y(n),vec2:s(r),vec3:u(i),vec4:S(a)}}const{mat3:w,mat4:T,quat:E,vec2:D,vec3:O,vec4:k}=C(Float32Array,Float32Array,Float32Array,Float32Array,Float32Array,Float32Array),{mat3:A,mat4:j,quat:M,vec2:N,vec3:P,vec4:F}=C(Float64Array,Float64Array,Float64Array,Float64Array,Float64Array,Float64Array),{mat3:I,mat4:ee,quat:L,vec2:R,vec3:te,vec4:ne}=C(r,Array,Array,Array,Array,Array);var re=/* @__PURE__ */ t({D:()=>B,MAX_LAT:()=>ie,R:()=>z,flatOf:()=>pe,globeCentre:()=>ge,globeOf:()=>fe,insideBy:()=>U,intersects:()=>ue,latOfMercY:()=>V,lonLatOf:()=>he,mercY:()=>ae,metresBetween:()=>_e,tileBBox:()=>se,tileBounds:()=>oe,tileOf:()=>ce,tilePoint:()=>le,worldAtUnwrapped:()=>me,worldOf:()=>de,wrap:()=>H});const z=6378137,B=Math.PI/180,ie=85.05112878,ae=e=>Math.log(Math.tan(Math.PI/4+e*B/2)),V=e=>(2*Math.atan(Math.exp(e))-Math.PI/2)/B,H=e=>((e+180)%360+360)%360-180;function oe(e,t,n){let r=2**e;return{w:t/r*360-180,e:(t+1)/r*360-180,n:Math.PI*(1-2*n/r),s:Math.PI*(1-2*(n+1)/r)}}function se(e,t,n){let r=oe(e,t,n);return[r.w,V(r.s),r.e,V(r.n)]}const ce=(e,t,n)=>{let r=2**n,i=ae(Math.max(-85.05112878,Math.min(ie,t)));return[Math.min(r-1,Math.max(0,Math.floor((e+180)/360*r))),Math.min(r-1,Math.max(0,Math.floor((Math.PI-i)/(2*Math.PI)*r)))]};function le(e,t,n,r,i){let a=oe(e,t,n);return[a.w+(a.e-a.w)*r,V(a.n+(a.s-a.n)*i)]}const ue=(e,t)=>e[0]<t[2]&&e[2]>t[0]&&e[1]<t[3]&&e[3]>t[1];function U(e,t,n){let r=111320*Math.cos(t*B),i=110574;return Math.min((e-n[0])*r,(n[2]-e)*r,(t-n[1])*i,(n[3]-t)*i)}function de(e,t,n,r=0){let i=fe(e,t,n,r),a=pe(e,t,n,r),o=e.morph;return[i[0]+(a[0]-i[0])*o,i[1]+(a[1]-i[1])*o,i[2]+(a[2]-i[2])*o]}function fe(e,t,n,r=0){let i=e.lat*B,a=n*B,o=H(t-e.lon)*B,s=a-i,c=z+r,l=e=>{let t=Math.sin(e/2);return 2*t*t},u=l(s)+Math.cos(i)*Math.cos(a)*l(o);return[c*Math.cos(a)*Math.sin(o),r-c*u,-c*(Math.sin(s)+Math.sin(i)*Math.cos(a)*l(o))]}function pe(e,t,n,r=0){let i=z*Math.cos(e.lat*B);return[i*H(t-e.lon)*B,r,-i*(ae(Math.max(-85.05112878,Math.min(ie,n)))-ae(e.lat))]}function me(e,t,n,r=0){let i=fe(e,t,n,r),a=pe(e,e.lon,n,r),o=e.morph;return a[0]=z*Math.cos(e.lat*B)*(t-e.lon)*B,[i[0]+(a[0]-i[0])*o,i[1]+(a[1]-i[1])*o,i[2]+(a[2]-i[2])*o]}function he(e,t,n,r){if(e.morph>.5){let n=z*Math.cos(e.lat*B);return[H(e.lon+t/n/B),V(ae(e.lat)-r/n)]}let i=e.lat*B,a=e.lon*B,o=[-Math.sin(a),Math.cos(a),0],s=[-Math.sin(i)*Math.cos(a),-Math.sin(i)*Math.sin(a),Math.cos(i)],c=[Math.cos(i)*Math.cos(a),Math.cos(i)*Math.sin(a),Math.sin(i)],l=[0,1,2].map(e=>t*o[e]+(n+z)*c[e]-r*s[e]);return[Math.atan2(l[1],l[0])/B,Math.atan2(l[2],Math.hypot(l[0],l[1]))/B]}const ge=()=>[0,-6378137,0];function _e(e,t){let n=e[1]*B,r=t[1]*B,i=r-n,a=H(t[0]-e[0])*B,o=Math.sin(i/2)**2+Math.cos(n)*Math.cos(r)*Math.sin(a/2)**2;return 2*z*Math.asin(Math.min(1,Math.sqrt(o)))}var W=class extends Error{code;fix;where;details;constructor(e,t,n={}){super(`aslnut ${e}: ${t}${n.where?` (at ${n.where})`:``}${n.fix?`. Fix: ${n.fix}`:``}`,n.cause===void 0?void 0:{cause:n.cause}),this.name=`AslnutError`,this.code=e,this.fix=n.fix??``,this.where=n.where??``,this.details=n.details}},ve=class extends W{asked;found;constructor(e,t){let n=!t.webgpu&&!t.webgl2;super(n?`E-GPU-001`:`E-GPU-002`,n?`this browser has neither WebGPU nor WebGL2 (${t.reason})`:`graphics '${e}' was asked for, but this browser has only ${t.webgpu?`WebGPU`:`WebGL2`} (${t.reason})`,{fix:n?`update the browser, or turn on hardware acceleration in its settings`:`use graphics: 'auto' (it picks ${t.webgpu?`WebGPU`:`WebGL2`} here)`,details:t}),this.name=`GraphicsUnavailable`,this.asked=e,this.found=t}};const ye=Object.freeze([{name:`world`,from:-1/0},{name:`country`,from:3.5},{name:`region`,from:6.5},{name:`city`,from:9.5},{name:`district`,from:12.5},{name:`street`,from:14.5},{name:`building`,from:17},{name:`room`,from:19}]),be=Object.freeze({margin:.3,dwell:.5,ahead:1,keep:Object.freeze({levels:2,seconds:60})}),xe={level:null,index:-1,levels:[],by:null};function Se(e){if(e==="default")return ye.map(e=>({...e}));if(!Array.isArray(e)||!e.length)throw new W(`E-LEVEL-001`,`observation levels: give a list of levels, or 'default'`,{where:`observation.levels`,fix:`levels: 'default' · levels: ['city', 'district', 'street'] · levels: [{ name: 'campus', from: 0 }, { name: 'room', from: 19 }]`});let t=[];for(let n of e)if(typeof n==`string`){let e=ye.find(e=>e.name===n);if(!e)throw new W(`E-LEVEL-001`,`observation levels: '${n}' is not a default level`,{where:`observation.levels`,fix:`a default level (${ye.map(e=>e.name).join(`, `)}), or your own { name, from }`});t.push({...e})}else if(n&&typeof n.name==`string`&&typeof n.from==`number`)t.push({name:n.name,from:n.from});else throw new W(`E-LEVEL-001`,`observation levels: ${JSON.stringify(n)} is not a level`,{where:`observation.levels`,fix:`a name ('street') or { name, from: zoom }`});if(t.sort((e,t)=>e.from-t.from),t[0]={name:t[0].name,from:-1/0},new Set(t.map(e=>e.name)).size!==t.length)throw new W(`E-LEVEL-001`,`observation levels: a name twice`,{where:`observation.levels`,fix:`give each level its own name`});return t}var Ce=class{ladder;names;margin;dwell;ahead;keep;bind;index;by=`orbit`;orbitAt;pending=null;recent=[];stateNow;constructor(e,t,n){if(this.ladder=Se(e.levels??`default`),this.names=this.ladder.map(e=>e.name),this.margin=Math.max(0,e.margin??be.margin),this.dwell=Math.max(0,e.dwell??be.dwell),this.ahead=Math.max(0,e.ahead??be.ahead),this.keep={levels:Math.max(0,e.keep?.levels??be.keep.levels),seconds:Math.max(0,e.keep?.seconds??be.keep.seconds)},this.bind=e.bind===void 0?`orbit`:e.bind,this.orbitAt=this.rungOf(t),this.index=this.orbitAt,typeof this.bind==`function`&&n){let e=this.bound(n);e!==null&&(this.index=e,this.by=`bind`)}e.level!==void 0&&(this.index=this.indexOf(e.level,`observation.level`),this.by=`app`),this.stateNow=this.makeState()}get level(){return this.ladder[this.index].name}get state(){return this.stateNow}rungOf(e){let t=0;for(;t+1<this.ladder.length&&e>=this.ladder[t+1].from;)t++;return t}indexOf(e,t=`level`){let n=this.names.indexOf(e);if(n<0)throw new W(`E-LEVEL-001`,`no level '${e}'`,{where:t,fix:`the levels: ${this.names.join(`, `)}`});return n}through(e){let t=this.orbitAt,n=this.ladder[t].from;return e>=(t+1<this.ladder.length?this.ladder[t+1].from:1/0)+this.margin?this.rungOf(e-this.margin):e<n-this.margin?this.rungOf(e+this.margin):t}bound(e){let t=this.bind,n;try{n=t(e)}catch(e){throw new W(`E-LEVEL-001`,`observation bind threw: ${e.message}`,{where:`observation.bind`,fix:`return a level name (or null: no change)`,cause:e})}return n==null?null:this.indexOf(n,`observation.bind`)}step(e,t,n){if(this.bind===!1)return this.pending=null,null;let r=typeof this.bind==`function`?(n?this.bound(n):null)??this.index:this.through(e);return r===(typeof this.bind==`function`?this.index:this.orbitAt)?(this.pending=null,null):(!this.pending||this.pending.to!==r)&&(this.pending={to:r,since:t},this.dwell>0)||t-this.pending.since<this.dwell*1e3||(this.pending=null,typeof this.bind!=`function`&&(this.orbitAt=r),r===this.index)?null:this.go(r,typeof this.bind==`function`?`bind`:`orbit`)}get waiting(){return this.pending?this.pending.since+this.dwell*1e3:null}set(e){let t=this.indexOf(e);return t===this.index?(this.by=`app`,null):this.go(t,`app`)}go(e,t){let n=this.level;this.index=e,this.by=t,this.recent.unshift(n);let r=/* @__PURE__ */ new Set;for(let e=0;e<this.recent.length;e++){let t=this.recent[e];r.has(t)||t===this.level?this.recent.splice(e--,1):r.add(t)}return this.recent.length>8&&(this.recent.length=8),this.stateNow=this.makeState(),{from:n,to:this.level,by:t}}near(e){if(this.bind!==`orbit`||this.ahead<=0)return null;let t=this.orbitAt,n=this.ladder[t].from;return e>=(t+1<this.ladder.length?this.ladder[t+1].from:1/0)-this.ahead?t+1:e<n+this.ahead&&t>0?t-1:null}stateAt(e){return{level:this.ladder[e].name,index:e,levels:this.names,by:this.by}}makeState(){return{level:this.level,index:this.index,levels:this.names,by:this.by}}matches(e,t,n=``){return we(e,this.names,t,n)}};function we(e,t,n,r=``){let i=e=>{let n=t.indexOf(e);if(n<0)throw new W(`E-LEVEL-001`,`no level '${e}'`,{where:r,fix:t.length?`the levels: ${t.join(`, `)}`:`this world has no levels: world(canvas, { observation: { levels: 'default' } })`});return n};if(typeof e==`string`)return i(e)===n;if(Array.isArray(e))return e.some(e=>i(e)===n);let a=e;if(!a||typeof a!=`object`||a.from===void 0&&a.to===void 0)throw new W(`E-WHEN-001`,`'when: { level }' is ${JSON.stringify(e)}`,{where:r,fix:`a level ('street'), levels (['city', 'district']) or a range ({ from: 'district' } · { from: 'city', to: 'street' })`});let o=a.from===void 0?0:i(a.from),s=a.to===void 0?t.length-1:i(a.to);return n>=Math.min(o,s)&&n<=Math.max(o,s)}const Te={linear:e=>e,in:e=>e*e*e,out:e=>1-(1-e)**3,inOut:e=>e<.5?4*e*e*e:1-(-2*e+2)**3/2,step:e=>+(e>=1)},Ee={linear:`linear`,smooth:`inOut`,cinematic:`cubic-bezier(0.7,0,0.18,1)`,glide:`linear`,snap:`cubic-bezier(0.15,0.85,0.25,1)`,anticipate:`cubic-bezier(0.55,-0.3,0.3,1)`,overshoot:`cubic-bezier(0.3,1.35,0.55,1)`},De={linear:`constant speed`,smooth:`cubic in-out`,cinematic:`slow start, long settle`,glide:`no stop: constant speed through the key`,snap:`fast in, soft settle`,anticipate:`a small pull back before moving`,overshoot:`passes the key, settles back`},Oe=e=>e<0?0:e>1?1:e;function ke(e,t,n,r){let i=t=>3*e*t*(1-t)**2+3*n*t*t*(1-t)+t**3,a=e=>3*t*e*(1-e)**2+3*r*e*e*(1-e)+e**3,o=t=>3*e*(1-t)**2+6*(n-e)*t*(1-t)+3*(1-n)*t*t;return e=>{if(e<=0)return 0;if(e>=1)return 1;let t=e;for(let n=0;n<6;n++){let n=o(t);if(Math.abs(n)<1e-6)break;t-=(i(t)-e)/n}if(t<0||t>1||Math.abs(i(t)-e)>1e-5){let n=0,r=1;t=e;for(let a=0;a<40;a++)t=(n+r)/2,i(t)<e?n=t:r=t}return a(t)}}function Ae(e=170,t=26){let n=Math.sqrt(Math.max(1e-6,e)),r=t/(2*n),i=Math.min(10,6.5/(Math.max(.05,Math.min(r,1))*n)),a=e=>{if(r<1){let t=n*Math.sqrt(1-r*r);return 1-Math.exp(-r*n*e)*(Math.cos(t*e)+r*n/t*Math.sin(t*e))}if(r===1)return 1-Math.exp(-n*e)*(1+n*e);let t=-n*(r-Math.sqrt(r*r-1)),i=-n*(r+Math.sqrt(r*r-1));return 1-(i*Math.exp(t*e)-t*Math.exp(i*e))/(i-t)};return e=>e<=0?0:e>=1?1:a(e*i)}const je=/* @__PURE__ */ new Map;function Me(e){if(typeof e==`function`)return e;let t=(e??`smooth`).trim(),n=Ee[t]??t,r=Te[n]??je.get(n);if(r)return r;let i=/^cubic-bezier\(\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*,\s*([-\d.]+)\s*\)$/.exec(n);if(i){let e=ke(+i[1],+i[2],+i[3],+i[4]);return je.set(n,e),e}if(i=/^steps(?:\(\s*(\d+)\s*\)|\s+(\d+))$/.exec(n),i){let e=Number(i[1]??i[2]),t=t=>t>=1?1:Math.floor(t*e)/e;return je.set(n,t),t}if(i=/^spring(?:\(\s*([\d.]+)\s*(?:,\s*([\d.]+)\s*)?\))?$/.exec(n),i){let e=Ae(i[1]?+i[1]:void 0,i[2]?+i[2]:void 0);return je.set(n,e),e}throw new W(`E-VIEW-001`,`unknown ease '${t}'`,{fix:`use a preset (${Object.keys(Ee).join(`, `)}), in / out / inOut, steps(n), cubic-bezier(a,b,c,d), spring(k, d) or a function`})}const Ne=(e,t)=>Me(e)(Oe(t)),Pe=(e,t=32)=>Array.from({length:t+1},(n,r)=>[r/t,Ne(e,r/t)]);function Fe(e,t,n,r,i=[0,0,0]){let a=e.dist*Math.cos(e.pitch);e.eye[0],e.groundY,e.eye[2];let o=Math.min(r,Math.max(60,.85*a+.6*e.dist));if(.55*a+.6*e.dist>r*2.2)return null;let s=[0,e.groundY,0],c=2*o+600,l=O.normalize(O.fromValues(t[0],Math.max(.02,t[1]),t[2])),u=Math.abs(l[1])>.99?[0,0,-1]:[0,1,0],d=O.fromValues(s[0],s[1],s[2]),f=O.add(d,O.mulScalar(l,c)),p=T.lookAt(f,d,u),m=2*o/n,h=O.transformMat4(d,p),g=-l[0],_=-l[1],v=-l[2],y=_*u[2]-v*u[1],b=v*u[0]-g*u[2],x=g*u[1]-_*u[0],S=Math.hypot(y,b,x)||1;y/=S,b/=S,x/=S;let C=b*v-x*_,w=x*g-y*v,E=y*_-b*g,D=i[0]*y+i[1]*b+i[2]*x,k=i[0]*C+i[1]*w+i[2]*E,A=Math.round((h[0]+D)/m)*m-D-h[0],j=Math.round((h[1]+k)/m)*m-k-h[1],M=T.ortho(-o+h[0]+A,o+h[0]+A,-o+h[1]+j,o+h[1]+j,1,2*c);return{viewProj:T.multiply(M,p),eye:[f[0],f[1],f[2]],texel:m,depthPerMetre:1/(2*c-1)}}function Ie(e,t){let n=63;for(let r=0;r<8;r++){let i=r&1?t[3]:t[0],a=r&2?t[4]:t[1],o=r&4?t[5]:t[2],s=e[0]*i+e[4]*a+e[8]*o+e[12],c=e[1]*i+e[5]*a+e[9]*o+e[13],l=e[2]*i+e[6]*a+e[10]*o+e[14],u=e[3]*i+e[7]*a+e[11]*o+e[15],d=0;if(s<-u&&(d|=1),s>u&&(d|=2),c<-u&&(d|=4),c>u&&(d|=8),l<0&&(d|=16),l>u&&(d|=32),n&=d,!n)return!0}return n===0}function Le(e,t={}){if(!(e>0))return 0;let n=Math.max(t.min??4,e/Math.max(.05,t.stepPx??1));return n>=60?1/0:n}function Re(e){let t=Math.abs(e)*360/86400;if(t<=0)return 0;let n=Math.max(1,t/.02);return n>=60?1/0:n}const ze=e=>e===1/0||e>=60?0:1e3/e,Be={open:0,told:/* @__PURE__ */ new Map};function Ve(){Be.open++}function He(){if(!(!Be.open||--Be.open))for(;Be.told.size;){let e=[...Be.told.values()];Be.told.clear();for(let t of e)try{t()}catch(e){console.error(e)}}}function Ue(e,t){Be.open?(Be.told.delete(e),Be.told.set(e,t)):t()}const We=(e,t,n)=>new W(`E-DATA-002`,e,{where:t,fix:n});function Ge(e,t={}){let n=t.where??`data`,r=Array.isArray(e)?e:e&&typeof e==`object`&&e.type===`FeatureCollection`?e.features:e&&typeof e==`object`&&e.type===`Feature`?[e]:e&&typeof e==`object`&&typeof e.type==`string`?[{type:`Feature`,geometry:e,properties:{}}]:(()=>{throw We(`not GeoJSON (got ${e===null?`null`:typeof e})`,n,`give a FeatureCollection, a Feature, a geometry or an array of features`)})(),i=[],a=/* @__PURE__ */ new Set;return r.forEach((e,r)=>{let o=e;if(!o||typeof o!=`object`)throw We(`feature ${r} is not an object`,n,`each feature is { type: "Feature", geometry, properties }`);let s=o.properties??o.props??{},c=typeof t.id==`function`?t.id(o,r):typeof t.id==`string`?s[t.id]:o.id;(c==null||c===``)&&(c=r),typeof c!=`string`&&typeof c!=`number`&&(c=String(c)),a.has(c)&&(c=`${c}#${r}`),a.add(c),i.push({id:c,props:s,geometry:qe(o.geometry,`${n} feature ${String(c)}`),...o.cuts?{cuts:o.cuts}:{}})}),i}const Ke=e=>Array.isArray(e)&&e.length>=2&&typeof e[0]==`number`&&typeof e[1]==`number`&&Number.isFinite(e[0])&&Number.isFinite(e[1]);function qe(e,t){if(e==null)return null;let n=e,r=e=>We(`${n.type}: ${e}`,t,`coordinates are [lon, lat] pairs, as in GeoJSON`);switch(n.type){case`Point`:if(!Ke(n.coordinates))throw r(`not a position`);break;case`MultiPoint`:case`LineString`:if(!Array.isArray(n.coordinates)||!n.coordinates.every(Ke))throw r(`not a list of positions`);break;case`MultiLineString`:case`Polygon`:if(!Array.isArray(n.coordinates)||!n.coordinates.every(e=>Array.isArray(e)&&e.every(Ke)))throw r(`not a list of lists of positions`);break;case`MultiPolygon`:if(!Array.isArray(n.coordinates)||!n.coordinates.every(e=>Array.isArray(e)&&e.every(e=>Array.isArray(e)&&e.every(Ke))))throw r(`not a list of polygons`);break;case`GeometryCollection`:{let e=(n.geometries??[]).map(e=>qe(e,t)).filter(Boolean),r=e.flatMap(e=>e.type===`Polygon`?[e.coordinates]:e.type===`MultiPolygon`?e.coordinates:[]);if(r.length)return{type:`MultiPolygon`,coordinates:r};let i=e.flatMap(e=>e.type===`LineString`?[e.coordinates]:e.type===`MultiLineString`?e.coordinates:[]);if(i.length)return{type:`MultiLineString`,coordinates:i};let a=e.flatMap(e=>e.type===`Point`?[e.coordinates]:e.type===`MultiPoint`?e.coordinates:[]);return a.length?{type:`MultiPoint`,coordinates:a}:null}default:throw We(`unknown geometry type '${n.type}'`,t,`use Point, LineString, Polygon or their Multi forms`)}return n}function Je(e){return e?e.type===`Polygon`?[e.coordinates]:e.type===`MultiPolygon`?e.coordinates:[]:[]}function Ye(e,t=!0){if(!e)return[];switch(e.type){case`LineString`:return[e.coordinates];case`MultiLineString`:return e.coordinates;case`Polygon`:return t?e.coordinates:[];case`MultiPolygon`:return t?e.coordinates.flat():[];default:return[]}}function Xe(e,t=!1){if(!e)return[];if(e.type===`Point`)return[e.coordinates];if(e.type===`MultiPoint`)return e.coordinates;if(!t)return[];let n=Qe(e);return n?[n]:[]}function Ze(e,t){if(!e)return t??null;let n=t??[1/0,1/0,-1/0,-1/0],r=e=>{e[0]<n[0]&&(n[0]=e[0]),e[1]<n[1]&&(n[1]=e[1]),e[0]>n[2]&&(n[2]=e[0]),e[1]>n[3]&&(n[3]=e[1])};switch(e.type){case`Point`:r(e.coordinates);break;case`MultiPoint`:case`LineString`:e.coordinates.forEach(r);break;case`MultiLineString`:case`Polygon`:e.coordinates.forEach(e=>e.forEach(r));break;case`MultiPolygon`:e.coordinates.forEach(e=>e.forEach(e=>e.forEach(r)))}return n}function Qe(e){if(!e)return null;if(e.type===`Point`)return e.coordinates;if(e.type===`MultiPoint`)return e.coordinates[0]??null;let t=Je(e);if(t.length){let e=null,n=-1;for(let r of t){let t=Math.abs($e(r[0]));t>n&&(n=t,e=r[0])}if(!e)return null;let r=0,i=0,a=0;for(let t=0,n=e.length-1;t<e.length;n=t++){let o=e[t],s=e[n],c=s[0]*o[1]-o[0]*s[1];a+=c,r+=(s[0]+o[0])*c,i+=(s[1]+o[1])*c}if(Math.abs(a)<1e-18)return e[0]??null;let o=[r/(3*a),i/(3*a)];return et(o,e)?o:e[Math.floor(e.length/2)]}let n=Ye(e);if(n.length){let e=n.reduce((e,t)=>t.length>e.length?t:e);return e[Math.floor(e.length/2)]??null}return null}function $e(e){let t=0;for(let n=0,r=e.length-1;n<e.length;r=n++)t+=(e[r][0]-e[n][0])*(e[r][1]+e[n][1]);return t/2}function et(e,t){let n=!1;for(let r=0,i=t.length-1;r<t.length;i=r++){let a=t[r],o=t[i];a[1]>e[1]!=o[1]>e[1]&&e[0]<(o[0]-a[0])*(e[1]-a[1])/(o[1]-a[1])+a[0]&&(n=!n)}return n}function tt(e){let t=Je(e);if(!t.length)return Qe(e);let n=null,r=-1;for(let e of t){let t=Math.abs($e(e[0]));t>r&&(r=t,n=e)}if(!n)return null;let i=n[0],a=Math.cos((i[0]?.[1]??0)*Math.PI/180),o=1/0,s=1/0,c=-1/0,l=-1/0;for(let e of i)e[0]<o&&(o=e[0]),e[0]>c&&(c=e[0]),e[1]<s&&(s=e[1]),e[1]>l&&(l=e[1]);let u=e=>et(e,i)&&!n.slice(1).some(t=>et(e,t)),d=e=>{let t=1/0;for(let r of n)for(let n=0,i=r.length-1;n<r.length;i=n++){let o=r[i][0]*a,s=r[i][1],c=r[n][0]*a,l=r[n][1],u=e[0]*a,d=e[1],f=c-o,p=l-s,m=f*f+p*p,h=m>0?Math.max(0,Math.min(1,((u-o)*f+(d-s)*p)/m)):0,g=o+h*f-u,_=s+h*p-d,v=g*g+_*_;v<t&&(t=v)}return Math.sqrt(t)},f=Qe({type:`Polygon`,coordinates:n}),p=Math.min((c-o)*a,l-s);if(f&&u(f)&&d(f)>p*.2)return f;let m=f&&u(f)?f:null,h=m?d(m):-1;for(let e=0;e<12;e++)for(let t=0;t<12;t++){let n=[o+(e+.5)/12*(c-o),s+(t+.5)/12*(l-s)];if(!u(n))continue;let r=d(n);r>h&&(h=r,m=n)}return m??f??i[0]??null}function nt(e,t){for(let n of Je(e))if(et(t,n[0])&&!n.slice(1).some(e=>et(t,e)))return!0;return!1}const rt=(e,t)=>e>=t[0]&&e<t[1],it=(e,t)=>t[0]<=t[1]?rt(e,t):e>=t[0]||e<t[1],at=[`level`,`hour`,`year`,`month`,`api`,`quality`,`role`,`toggle`],ot=[`zoom`,`dist`,`altitude`,`tilt`];function st(e,t,n=``){if(e===void 0||e===!0)return 1;if(e===!1)return 0;if(typeof e==`function`){let r;try{r=e(t)}catch(e){throw new W(`E-WHEN-001`,`a 'when' function threw: ${e.message}`,{where:n,fix:`check the fields it reads exist on s (s.observation, s.time, s.world, s.user, s.device, s.render)`,cause:e})}if(typeof r==`boolean`)return+!!r;if(typeof r==`number`&&Number.isFinite(r))return r<0?0:r>1?1:r;throw new W(`E-WHEN-001`,`a 'when' function returned ${typeof r==`number`?r:typeof r}`,{where:n,fix:`return a boolean, or a number from 0 to 1`})}if(typeof e==`object`){for(let t of Object.keys(e)){if(ot.includes(t))throw new W(`E-WHEN-001`,`'when: { ${t} }' is gone: the camera never decides what is shown`,{where:n,fix:`use the observation level: when: { level: { from: 'district' } } (world(canvas, { observation: { levels: 'default' } })); a GIS layer needs no gate at all`});if(!at.includes(t))throw new W(`E-WHEN-001`,`unknown 'when' key '${t}'`,{where:n,fix:`use one of ${at.join(`, `)}, or a function s => …`})}let r=e,i=t.time;if(r.level!==void 0){let e=t.observation;if(!e||e.level===null)throw new W(`E-WHEN-001`,`'when: { level }' in a world with no levels`,{where:n,fix:`world(canvas, { observation: { levels: 'default' } }) (a world turns the default ladder on by itself when an element asks for a level)`});if(!we(r.level,e.levels,e.index,n))return 0}return r.hour&&!it(i.hour,r.hour)||r.year&&!rt(i.year,r.year)||r.month&&!it(i.month,r.month)||r.api&&!(Array.isArray(r.api)?r.api:[r.api]).includes(t.device.api)||r.quality&&t.render.quality!==r.quality||r.role&&!(Array.isArray(r.role)?r.role:[r.role]).includes(t.user.role)||r.toggle&&!t.user.toggles[r.toggle]?0:1}throw new W(`E-WHEN-001`,`'when' is a ${typeof e}`,{where:n,fix:`use a boolean, an object like { level: 'street' } or { hour: [18, 6] }, or a function s => …`})}function ct(e,t,n=``){return new Proxy(e,{get(e,r,i){let a=Reflect.get(e,r,i);if(typeof r!=`string`)return a;let o=n?`${n}\u0001${r}`:r;return a&&typeof a==`object`&&!(a instanceof Date)&&!Array.isArray(a)?ct(a,t,o):(t.add(o),a)}})}function lt(e,t){let n=``;for(let r of t){let t=e;for(let e of r.split(``)){if(t==null)break;t=t[e]}n+=`${r}=${t instanceof Date?t.getTime():Array.isArray(t)?t.join(``):t&&typeof t==`object`?JSON.stringify(t):String(t)}\u0003`}return n}function ut(e,t){if(typeof e==`boolean`)return+!!e;if(typeof e==`number`&&Number.isFinite(e))return e<0?0:e>1?1:e;throw new W(`E-WHEN-001`,`a 'when' function returned ${typeof e==`number`?e:typeof e}`,{where:t,fix:`return a boolean, or a number from 0 to 1`})}const dt=(e,t)=>typeof e==`function`?e(t):e;function ft(e,t){if(e===void 0||typeof e==`boolean`)return!1;if(typeof e==`object`)return e.level!==void 0;let n=/* @__PURE__ */ new Set;try{e(ct(t,n))}catch{return!1}for(let e of n)if(e===`observation`||e.startsWith(`observation`))return!0;return!1}const pt=e=>{if(typeof e!=`object`||!e||Array.isArray(e))return!1;let t=Object.getPrototypeOf(e);return t===Object.prototype||t===null};function mt(e){if(!pt(e))return!1;let t=Object.keys(e);return t.length===1&&t[0]===`level`&&pt(e.level)}function ht(e,t,n,r=``){let i=Object.keys(e.level);if(!i.length)throw new W(`E-LEVEL-002`,`${r||`a prop by level`}: no levels named`,{where:r,fix:`name at least one level: { level: { district: 6, street: 14 } }`});let a=-1,o=1/0;for(let e of i){let i=t.indexOf(e);if(i<0)throw new W(`E-LEVEL-002`,`${r||`a prop by level`}: no level '${e}' on the ladder`,{where:r,fix:t.length?`the levels: ${t.join(`, `)}`:`this world has no levels: world(canvas, { observation: { levels: 'default' } })`});i<=n?i>a&&(a=i):i<o&&(o=i)}return e.level[t[a>=0?a:o]]}function gt(e){if(typeof e==`number`)return e>>>0;if(typeof e==`string`){let t=e.replace(`#`,``),n=t.length===3?t.split(``).map(e=>e+e).join(``):t.slice(0,6);return parseInt(n,16)>>>0}return(Math.round(Math.max(0,Math.min(1,e[0]))*255)<<16)+(Math.round(Math.max(0,Math.min(1,e[1]))*255)<<8)+Math.round(Math.max(0,Math.min(1,e[2]))*255)}function _t(e,t,n=.06){let r=gt(e),i=1+(t*2-1)*n,a=e=>Math.max(0,Math.min(255,Math.round((r>>e&255)*i)));return(a(16)<<16)+(a(8)<<8)+a(0)}function vt(e,t,n){let r=e,i=n,a=t,o=Math.abs(r)+Math.abs(i)+Math.abs(a)||1,s=r/o,c=i/o;if(a<0){let e=(1-Math.abs(c))*(s>=0?1:-1),t=(1-Math.abs(s))*(c>=0?1:-1);s=e,c=t}let l=Math.round((s*.5+.5)*4095),u=Math.round((c*.5+.5)*4095);return l*4096+u}function yt(e){let t=Math.round(e),n=(t>>12)/4095*2-1,r=(t&4095)/4095*2-1,i=n,a=r,o=1-Math.abs(n)-Math.abs(r),s=Math.max(-o,0);i+=i>=0?-s:s,a+=a>=0?-s:s;let c=Math.hypot(i,o,a)||1;return[i/c,o/c,a/c]}const bt=vt(0,1,0),xt=8;var St=class{d=/* @__PURE__ */ new Float32Array(256);n=0;ex=/* @__PURE__ */ new Float64Array(32);tintNow=!1;matNow=0;idNow=0;anchor=null;get count(){return this.n/8}as(e){return this.idNow=e,this}material(e){return this.matNow=e,this}on(e){return this.anchor=e,this}lift(e){return this.liftNow=e,this}liftNow=0;normalNow=null;v(e,t,n){let r=this.anchor,i=this.n;i+8>this.d.length&&this.room(i+8);let a=this.d;a[i]=e[0],a[i+1]=e[1]+this.liftNow,a[i+2]=e[2],a[i+3]=t,a[i+4]=r?r[0]:e[0],a[i+5]=r?r[1]:e[2],a[i+6]=this.normalNow??n,a[i+7]=this.idNow,this.ex[i/8]=this.cardNow||this.matNow+(this.tintNow?Tt:0),this.n=i+8}room(e){let t=this.d.length;for(;t<e;)t*=2;let n=new Float32Array(t),r=new Float64Array(t/8);n.set(this.d.subarray(0,this.n)),r.set(this.ex.subarray(0,this.n/8)),this.d=n,this.ex=r}tri(e,t,n,r){let i=t[0]-e[0],a=t[1]-e[1],o=t[2]-e[2],s=n[0]-e[0],c=n[1]-e[1],l=n[2]-e[2],u=a*l-o*c,d=o*s-i*l,f=i*c-a*s,p=Math.hypot(u,d,f);if(p<1e-12)return this;u/=p,d/=p,f/=p;let m=gt(r),h=Math.abs(d)>.9999&&d>0?bt:vt(u,d,f);return this.v(e,m,h),this.v(t,m,h),this.v(n,m,h),this}quad(e,t,n,r,i){return this.tri(e,t,n,i).tri(e,n,r,i)}wall(e,t,n,r,i,a,o,s=i,c=a){return this.quad([e,i,t],[n,s,r],[n,c,r],[e,a,t],o)}box(e,t=0,n=0,r=1,i=1,a=1,o=0,s=`#ffffff`,c){if(typeof e==`object`){let[t,n,r]=Ct(e.size,1),i=e.at??[0,0,0],a=e.color??`#ffffff`,o=Math.min(e.round??0,t/2,n/2,r/2);return o>0?this.rounded(i,[t/2,n/2,r/2],o,a):this.box(i[0],i[2],i[1]-n/2,t,n,r,0,a)}let l=e,u=Math.cos(o),d=Math.sin(o),f=r/2,p=a/2,m=(e,n)=>[l+e*u-n*d,t+e*d+n*u],h=[m(-f,-p),m(f,-p),m(f,p),m(-f,p)];for(let e=0;e<4;e++){let t=h[e],r=h[(e+1)%4];this.wall(r[0],r[1],t[0],t[1],n,n+i,s)}let g=c??s;return this.quad([h[0][0],n+i,h[0][1]],[h[3][0],n+i,h[3][1]],[h[2][0],n+i,h[2][1]],[h[1][0],n+i,h[1][1]],g),this}prism(e,t,n,r,i,a){let o=e.length,s=wt(e)>0;for(let i=0;i<o;i++){let a=e[i],c=e[(i+1)%o];s?this.wall(a[0],a[1],c[0],c[1],t,n,r):this.wall(c[0],c[1],a[0],a[1],t,n,r)}for(let t=0;t+2<a.length;t+=3){let r=e[a[t]],o=e[a[t+1]],s=e[a[t+2]];(o[1]-r[1])*(s[0]-r[0])-(o[0]-r[0])*(s[1]-r[1])>0?this.tri([r[0],n,r[1]],[o[0],n,o[1]],[s[0],n,s[1]],i):this.tri([r[0],n,r[1]],[s[0],n,s[1]],[o[0],n,o[1]],i)}return this}ribbon(e,t,n,r,i){let a=this.anchor;this.anchor=null;for(let a=0;a+1<e.length;a++){let[o,s]=e[a],[c,l]=e[a+1],u=Math.hypot(c-o,l-s);if(u<1e-6)continue;let d=-(l-s)/u,f=(c-o)/u,p=[o+d*t,r,s+f*t],m=[o+d*n,r,s+f*n],h=[c+d*t,r,l+f*t],g=[c+d*n,r,l+f*n];this.quadUp(p,m,g,h,i)}return this.anchor=a,this}floor(e,t,n,r){let i=this.anchor;this.anchor=null;for(let i=0;i+2<r.length;i+=3){let a=e[r[i]],o=e[r[i+1]],s=e[r[i+2]];this.triUp([a[0],t,a[1]],[o[0],t,o[1]],[s[0],t,s[1]],n)}return this.anchor=i,this}smoothTri(e,t,n,r,i,a,o){let s=gt(o);return this.v(e,s,vt(t[0],t[1],t[2])),this.v(n,s,vt(r[0],r[1],r[2])),this.v(i,s,vt(a[0],a[1],a[2])),this}card(e,t,n,r,i,a,o){let s=gt(o),c=vt(i[0],i[1],i[2]);this.idNow;let l=Math.max(1,Math.min(15,Math.round(a)))*65536,u=(e,t,n)=>{this.cardNow=1+t*255+n*255*256+l,this.v(e,s,c)};return u(e,0,0),u(t,1,0),u(n,1,1),u(e,0,0),u(n,1,1),u(r,0,1),this.cardNow=0,this}quadUp(e,t,n,r,i){return this.triUp(e,t,n,i).triUp(e,n,r,i)}triUp(e,t,n,r){let i=gt(r);return(t[2]-e[2])*(n[0]-e[0])-(t[0]-e[0])*(n[2]-e[2])>=0?(this.v(e,i,bt),this.v(t,i,bt),this.v(n,i,bt)):(this.v(e,i,bt),this.v(n,i,bt),this.v(t,i,bt)),this}blob(e,t,n,r,i,a,o,s=1){for(let[c,l,u]of At(s))this.tri([e+c[0]*r,t+c[1]*i,n+c[2]*a],[e+l[0]*r,t+l[1]*i,n+l[2]*a],[e+u[0]*r,t+u[1]*i,n+u[2]*a],o);return this}cylinder(e,t=0,n=0,r=1,i=1,a=1,o=8,s=`#ffffff`){if(typeof e==`object`){let t=e.radius,n=Ct(e.size,1)[1],r=e.at??[0,0,0],i=Math.max(3,e.sides??16),a=e.color??`#ffffff`,o=e.axis??`y`,s=e=>o===`y`?[r[0]+e[0],r[1]+e[1],r[2]+e[2]]:o===`x`?[r[0]+e[1],r[1]+e[0],r[2]+e[2]]:[r[0]+e[0],r[1]+e[2],r[2]+e[1]],c=o!==`y`;for(let e=0;e<i;e++){let r=e/i*Math.PI*2,o=(e+1)/i*Math.PI*2,l=Math.cos(r)*t,u=Math.sin(r)*t,d=Math.cos(o)*t,f=Math.sin(o)*t,p=n/2,m=s([d,-p,f]),h=s([l,-p,u]),g=s([l,p,u]),_=s([d,p,f]),v=s([0,p,0]),y=s([0,-p,0]);c?(this.quad(_,g,h,m,a),this.tri(v,g,_,a),this.tri(y,m,h,a)):(this.quad(m,h,g,_,a),this.tri(v,_,g,a),this.tri(y,h,m,a))}return this}let c=e;for(let e=0;e<o;e++){let l=e/o*Math.PI*2,u=(e+1)/o*Math.PI*2;this.quad([c+Math.cos(u)*i,n,t+Math.sin(u)*i],[c+Math.cos(l)*i,n,t+Math.sin(l)*i],[c+Math.cos(l)*a,r,t+Math.sin(l)*a],[c+Math.cos(u)*a,r,t+Math.sin(u)*a],s)}return this}sphere(e){let t=e.at??[0,0,0];return this.blob(t[0],t[1],t[2],e.radius,e.radius,e.radius,e.color??`#ffffff`,Math.max(0,Math.min(2,e.detail??1)))}rounded(e,t,n,r){let i=[t[0]-n,t[1]-n,t[2]-n],a=(t,r)=>{let a=t/16*Math.PI*2,o=r/8*Math.PI,s=[Math.sin(o)*Math.cos(a),Math.cos(o),Math.sin(o)*Math.sin(a)],c=[Math.sign(s[0])*i[0],Math.sign(s[1])*i[1],Math.sign(s[2])*i[2]];return[[e[0]+c[0]+s[0]*n,e[1]+c[1]+s[1]*n,e[2]+c[2]+s[2]*n],s]};for(let e=0;e<8;e++)for(let t=0;t<16;t++){let[n,i]=a(t,e),[o,s]=a(t+1,e),[c,l]=a(t+1,e+1),[u,d]=a(t,e+1);this.smoothTri(n,i,o,s,c,l,r).smoothTri(n,i,c,l,u,d,r)}return this}build(){let e=this.d.slice(0,this.n);for(let t=0,n=0;t<e.length;t+=8,n++){let r=this.ex[n]%Tt;r>0&&r<256&&(e[t+7]=e[t+7]+r*65536)}return e}adopt(e){return this.n+e.n>this.d.length&&this.room(this.n+e.n),this.d.set(e.d.subarray(0,e.n),this.n),this.ex.set(e.ex.subarray(0,e.n/8),this.n/8),this.n+=e.n,this}extras(){return Float32Array.from(this.ex.subarray(0,this.n/8))}solidVertex(e,t,n,r=0,i=!1){let a=this.matNow,o=this.tintNow;return this.matNow=r,this.tintNow=i,this.v(e,gt(n),vt(t[0],t[1],t[2])),this.matNow=a,this.tintNow=o,this}cardNow=0};const Ct=(e,t)=>typeof e==`number`?[e,e,e]:e?[e[0],e[1],e[2]]:[t,t,t];function wt(e){let t=0;for(let n=0,r=e.length-1;n<e.length;r=n++)t+=(e[r][0]+e[n][0])*(e[r][1]-e[n][1]);return t/2}const Tt=512,Et=6;function Dt(e){let t=new St;e(t);let n=t.build(),r=t.extras(),i=new Float32Array(n.length/8*6),a=!1;for(let e of r)if(e>=512&&e<65536){a=!0;break}for(let e=0,t=0,o=0;e<n.length;e+=8,t+=6,o++){let s=r[o];i[t]=n[e],i[t+1]=n[e+1],i[t+2]=n[e+2],i[t+3]=n[e+6],i[t+4]=n[e+3],i[t+5]=s>=65536?s:s>=512?s-512:s+(a?256:0)}return i}const Ot=10;let kt=[];function At(e){if(kt[e])return kt[e];let t=(1+Math.sqrt(5))/2,n=[[-1,t,0],[1,t,0],[-1,-t,0],[1,-t,0],[0,-1,t],[0,1,t],[0,-1,-t],[0,1,-t],[t,0,-1],[t,0,1],[-t,0,-1],[-t,0,1]].map(e=>{let t=Math.hypot(...e);return[e[0]/t,e[1]/t,e[2]/t]}),r=[[0,11,5],[0,5,1],[0,1,7],[0,7,10],[0,10,11],[1,5,9],[5,11,4],[11,10,2],[10,7,6],[7,1,8],[3,9,4],[3,4,2],[3,2,6],[3,6,8],[3,8,9],[4,9,5],[2,4,11],[6,2,10],[8,6,7],[9,8,1]].map(([e,t,r])=>[n[e],n[t],n[r]]);for(let t=0;t<e;t++){let e=[],t=(e,t)=>{let n=[(e[0]+t[0])/2,(e[1]+t[1])/2,(e[2]+t[2])/2],r=Math.hypot(...n);return[n[0]/r,n[1]/r,n[2]/r]};for(let[n,i,a]of r){let r=t(n,i),o=t(i,a),s=t(a,n);e.push([n,r,s],[i,o,r],[a,s,o],[r,o,s])}r=e}return kt[e]=r,kt[e]}const jt=[1,0,0,0,1,0,0,0,1,0,0,0],Mt=(e,t)=>{let n=Array(12);for(let r=0;r<4;r++)for(let i=0;i<3;i++)n[r*3+i]=e[i]*t[r*3]+e[3+i]*t[r*3+1]+e[6+i]*t[r*3+2]+(r===3?e[9+i]:0);return n},Nt=e=>[1,0,0,0,1,0,0,0,1,e[0],e[1],e[2]],Pt=e=>{let t=Math.cos(e*Math.PI/180),n=Math.sin(e*Math.PI/180);return[t,0,n,0,1,0,-n,0,t,0,0,0]},Ft=(e,t)=>{let n=-t*Math.PI/180,r=Math.cos(n),i=Math.sin(n),a=1-r,[o,s,c]=e;return[r+o*o*a,s*o*a+c*i,c*o*a-s*i,o*s*a-c*i,r+s*s*a,c*s*a+o*i,o*c*a+s*i,s*c*a-o*i,r+c*c*a,0,0,0]},It=e=>[e[0],0,0,0,e[1],0,0,0,e[2],0,0,0],Lt=(e,t,n,r)=>[e[0],e[1],e[2],t[0],t[1],t[2],n[0],n[1],n[2],r[0],r[1],r[2]],Rt=e=>e[0]*(e[4]*e[8]-e[5]*e[7])-e[3]*(e[1]*e[8]-e[2]*e[7])+e[6]*(e[1]*e[5]-e[2]*e[4]),G=(e,t)=>[e[0]-t[0],e[1]-t[1],e[2]-t[2]],K=(e,t,n=1)=>[e[0]+t[0]*n,e[1]+t[1]*n,e[2]+t[2]*n],zt=(e,t)=>[e[1]*t[2]-e[2]*t[1],e[2]*t[0]-e[0]*t[2],e[0]*t[1]-e[1]*t[0]],Bt=(e,t)=>e[0]*t[0]+e[1]*t[1]+e[2]*t[2],q=e=>{let t=Math.hypot(e[0],e[1],e[2])||1;return[e[0]/t,e[1]/t,e[2]/t]},Vt=e=>[-e[0],-e[1],-e[2]];var Ht=class{level;anchors=[];parts=[];groups=[];emitters=[];m=jt;mb;constructor(e,t=0){this.mb=e,this.level=t}part(e,t,n={}){let r=this.mb.count;t(this);let i=n.joint,a=i?typeof i.axis==`string`?{x:[1,0,0],y:[0,1,0],z:[0,0,1]}[i.axis]:i.axis:null;return this.parts.push({name:e,pivot:this.place(n.pivot??[0,0,0]),first:r,count:this.mb.count-r,...i&&a?{joint:{...i,axis:this.dir(a)}}:{}}),this}group(e,t){let n=this.mb.count;return t(this),this.groups.push({name:e,first:n,count:this.mb.count-n}),this}anchor(e,t,n={}){return this.anchors.push({name:e,at:this.place(t),kind:n.kind,heading:n.heading}),this}frame(e,t){let n=typeof e.scale==`number`?[e.scale,e.scale,e.scale]:e.scale??[1,1,1];return this.within(Mt(Mt(Nt(e.at??[0,0,0]),Pt(e.turn??0)),It(n)),()=>t(this))}within(e,t){let n=this.m;this.m=Mt(n,e);try{t()}finally{this.m=n}return this}place(e){let t=this.m;return[t[0]*e[0]+t[3]*e[1]+t[6]*e[2]+t[9],t[1]*e[0]+t[4]*e[1]+t[7]*e[2]+t[10],t[2]*e[0]+t[5]*e[1]+t[8]*e[2]+t[11]]}dir(e){let t=this.m;return q([t[0]*e[0]+t[3]*e[1]+t[6]*e[2],t[1]*e[0]+t[4]*e[1]+t[7]*e[2],t[2]*e[0]+t[5]*e[1]+t[8]*e[2]])}};const Ut=[`flat`,`gable`,`hip`,`dutch`,`skillion`,`pyramidal`],Wt=Math.PI/180;function Gt(e,t,n,r={}){let i=Kt(e,t,n,r),a=(i===`x`?t:n)/2,o=(i===`x`?n:t)/2,s=Math.tan((r.pitch??25)*Wt);return e===`flat`?{rise:r.thick??.25,ridge:`x`}:e===`skillion`?{rise:r.rise??2*o*s,ridge:i}:e===`pyramidal`?{rise:r.rise??Math.min(a,o)*s,ridge:i}:{rise:r.rise??o*s,ridge:i}}function Kt(e,t,n,r){if(e===`skillion`){let e=r.high??`+z`;return e===`+z`||e===`-z`?`x`:`z`}let i=r.ridge??`long`;return i===`x`||i===`z`?i:t>=n==(i===`long`)?`x`:`z`}function qt(e,t,n,r={}){let{rise:i,ridge:a}=Gt(e,t,n,r),o=(a===`x`?t:n)/2,s=(a===`x`?n:t)/2,c=Math.max(0,Math.min(r.endInset??0,o*.45)),l=(e,t,n)=>a===`x`?[e,t,n]:[n,t,e],u=[],d=[],f=[],p=[0,e===`flat`?i/2:i*.3,0],m=(e,t,n,r)=>{let i=[(n[1]-t[1])*(r[2]-t[2])-(n[2]-t[2])*(r[1]-t[1]),(n[2]-t[2])*(r[0]-t[0])-(n[0]-t[0])*(r[2]-t[2]),(n[0]-t[0])*(r[1]-t[1])-(n[1]-t[1])*(r[0]-t[0])];if(Math.hypot(i[0],i[1],i[2])<1e-12)return;let a=[(t[0]+n[0]+r[0])/3-p[0],(t[1]+n[1]+r[1])/3-p[1],(t[2]+n[2]+r[2])/3-p[2]];i[0]*a[0]+i[1]*a[1]+i[2]*a[2]>=0?e.push(t,n,r):e.push(t,r,n)},h=(e,...t)=>{for(let n=1;n+1<t.length;n++)m(e,t[0],t[n],t[n+1])},g=()=>{let e=l(-o,0,-s),t=l(o,0,-s),n=l(o,0,s),r=[e,t,n,e,n,l(-o,0,s)];for(let e=0;e<6;e+=3){let t=r[e],n=r[e+1],i=r[e+2];(n[2]-t[2])*(i[0]-t[0])-(n[0]-t[0])*(i[2]-t[2])<=0?f.push(t,n,i):f.push(t,i,n)}};if(e===`flat`){let e=i,t=(e,t,n)=>l(e,n,t);h(u,t(-o,-s,e),t(o,-s,e),t(o,s,e),t(-o,s,e));for(let[n,r,i,a]of[[-o,-s,o,-s],[o,-s,o,s],[o,s,-o,s],[-o,s,-o,-s]])h(u,t(n,r,0),t(i,a,0),t(i,a,e),t(n,r,e));return g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}if(e===`skillion`){let e=r.high??`+z`,t=e===`+z`||e===`+x`?1:-1,n=e=>e*t>0?i:0;h(u,l(-o,n(-s),-s),l(o,n(-s),-s),l(o,n(s),s),l(-o,n(s),s));let p=Math.min(c,s*.45),m=t*(s-p),_=-t*(s-p),v=i*(2*s-p)/(2*s),y=i*p/(2*s);for(let e of[-(o-c),o-c])h(d,l(e,0,_),l(e,0,m),l(e,v,m),l(e,y,_));return h(d,l(-(o-c),0,m),l(o-c,0,m),l(o-c,v,m),l(-(o-c),v,m)),g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}if(e===`pyramidal`){let e=l(0,i,0);return m(u,l(-o,0,-s),l(o,0,-s),e),m(u,l(o,0,-s),l(o,0,s),e),m(u,l(o,0,s),l(-o,0,s),e),m(u,l(-o,0,s),l(-o,0,-s),e),g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}if(e===`gable`){for(let e of[-1,1])h(u,l(-o,0,e*s),l(o,0,e*s),l(o,i,0),l(-o,i,0));for(let e of[-(o-c),o-c])m(d,l(e,0,-s),l(e,0,s),l(e,i,0));return g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}if(e===`hip`){let e=Math.max(0,o-s);for(let t of[-1,1])h(u,l(-o,0,t*s),l(o,0,t*s),l(e,i,0),l(-e,i,0)),m(u,l(t*o,0,-s),l(t*o,0,s),l(t*e,i,0));return g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}let _=Math.max(.2,Math.min(.8,r.gablet??.5)),v=o-_*s,y=(1-_)*s,b=_*i;for(let e of[-1,1])h(u,l(-v,0,e*s),l(v,0,e*s),l(v,i,0),l(-v,i,0)),m(u,l(v,0,e*s),l(o,0,e*s),l(v,b,e*y)),m(u,l(-o,0,e*s),l(-v,0,e*s),l(-v,b,e*y)),h(u,l(e*o,0,-s),l(e*o,0,s),l(e*v,b,y),l(e*v,b,-y)),m(d,l(e*v,b,-y),l(e*v,b,y),l(e*v,i,0));return g(),{slopes:u,ends:d,under:f,rise:i,ridge:a}}var Jt=class e{b;o;x;y;z;size;ring;constructor(e,t,n,r,i,a,o=null){this.b=e,this.o=t,this.x=n,this.y=r,this.z=i,this.size=a,this.ring=o}static footprint(t,n,r){let i=1/0,a=-1/0,o=1/0,s=-1/0;for(let[e,t]of n)i=Math.min(i,e),a=Math.max(a,e),o=Math.min(o,t),s=Math.max(s,t);return new e(t,[i,0,o],[1,0,0],[0,1,0],[0,0,1],[a-i,r,s-o],n)}get width(){return this.size[0]}get height(){return this.size[1]}get depth(){return this.size[2]}at(e,t,n,r=!1){let i=r?[1,1,1]:this.size;return K(K(K(this.o,this.x,e*i[0]),this.y,t*i[1]),this.z,n*i[2])}split(t,n){let r=t===`x`?0:t===`y`?1:2,i=this.size[r],a=n.reduce((e,t)=>e+(typeof t==`number`?t:0),0),o=n.filter(e=>e===`*`).length,s=[],c=0,l=t=>{let n=K(this.o,r===0?this.x:r===1?this.y:this.z,c),i=[...this.size];i[r]=t,s.push(new e(this.b,n,this.x,this.y,this.z,i)),c+=t};for(let e of n)if(typeof e==`number`)l(Math.max(0,e));else if(e===`*`)l(Math.max(0,(i-a)/Math.max(1,o)-0));else{let t=Number(e.slice(1)),r=Math.max(0,i-a-c+(n.indexOf(e),0)),o=Math.max(1,Math.round(r/t));for(let e=0;e<o;e++)l(r/o)}return s}repeat(e,t){return this.split(e,[`~${t}`])}faces(){let[t,n,r]=this.size,i=this.x,a=this.y,o=this.z,s=Vt(o),c=Vt(i),l=new e(this.b,K(this.o,o,r),i,a,o,[t,n,0]),u=new e(this.b,K(this.o,i,t),c,a,s,[t,n,0]),d=new e(this.b,K(K(this.o,i,t),o,r),s,a,i,[r,n,0]),f=new e(this.b,this.o,o,a,c,[r,n,0]);return{front:l,back:u,left:f,right:d,top:new e(this.b,K(K(this.o,a,n),o,r),i,s,a,[t,r,0]),bottom:new e(this.b,this.o,i,o,s,[t,r,0]),walls:this.ring?Yt(this):[l,d,u,f]}}get footprint(){return this.ring?[...this.ring]:[this.at(0,0,0),this.at(1,0,0),this.at(1,0,1),this.at(0,0,1)].map(e=>[e[0],e[2]])}solid(e=`box`,t={}){let n=t.inset??0,r=[Math.max(.001,this.size[0]-2*n),Math.max(.001,this.size[1]-2*n),Math.max(.001,this.size[2]-2*n)];if(this.ring&&e===`box`)return this.b.sweep(this.ring.map(([e,t])=>[e,t]),{up:this.size[1]},t),this;let i=this.at(.5,.5,.5);return this.b.within(Lt(this.x,this.y,this.z,i),()=>this.b.solid(e,{...t,size:r,at:[0,0,0]})),this}panel(e={}){let t=e.flat?0:e.depth??.05,n=e.inset??0,r=Math.max(.001,this.size[0]-2*n),i=Math.max(.001,this.size[1]-2*n),a=K(this.at(.5,.5,0),this.z,(e.out??0)+t/2);return this.b.within(Lt(this.x,this.y,this.z,a),()=>{if(e.flat){let t=r/2,n=i/2;this.b.mesh([[-t,-n,0],[t,-n,0],[t,n,0],[-t,-n,0],[t,n,0],[-t,n,0]],e)}else this.b.solid(`box`,{...e,size:[r,i,t],at:[0,0,0]})}),this}build(e){return this.b.within(Lt(this.x,this.y,this.z,this.o),()=>e(this.b,{width:this.size[0],height:this.size[1],depth:this.size[2]})),this}roof(e,t={}){let n=t.overhang??.4,r=qt(e,this.size[0]+2*n,this.size[2]+2*n,{pitch:t.pitch,ridge:t.ridge,rise:t.rise,gablet:t.gablet,high:t.high,endInset:n});return this.b.within(Lt(this.x,this.y,this.z,this.at(.5,1,.5)),()=>{this.b.mesh(r.slopes,t),r.ends.length&&this.b.mesh(r.ends,{...t,...t.ends}),this.b.mesh(r.under,{...t,detail:0})}),this}extrude(t){return new e(this.b,this.o,this.x,this.y,this.z,[this.size[0],this.size[1],t])}inset(t,n,r=t,i=n){return new e(this.b,K(K(this.o,this.x,t),this.y,n),this.x,this.y,this.z,[Math.max(0,this.size[0]-t-r),Math.max(0,this.size[1]-n-i),this.size[2]])}};function Yt(e){let t=e.footprint,n=e.size[1],r=[],i=0;for(let e=0,n=t.length-1;e<t.length;n=e++)i+=t[n][0]*t[e][1]-t[e][0]*t[n][1];for(let a=0;a<t.length;a++){let o=t[a],s=t[(a+1)%t.length],c=Math.hypot(s[0]-o[0],s[1]-o[1]);if(c<.05)continue;let l=[(s[0]-o[0])/c,0,(s[1]-o[1])/c],u=i>0?[-l[2],0,l[0]]:[l[2],0,-l[0]];r.push(new Jt(e.b,[o[0],e.o[1],o[1]],l,[0,1,0],u,[c,n,0]))}return r}function Xt(e){let t=e.start??Object.keys(e.rules)[0];return new Ar({id:e.id,params:e.params,far:e.far,budget:e.budget,build:(n,r,i)=>{let a=0,o=(t,n)=>{let s=e.rules[t];if(!s)throw new W(`E-DATA-001`,`the grammar has no rule '${t}' (it has ${Object.keys(e.rules).join(`, `)})`,{fix:`rules: { ${t}: (s, p, go) => … }`});if(++a>2e3)throw new W(`E-DATA-001`,`the grammar went ${a} rules deep (a rule that calls itself without end?)`,{fix:`split scopes smaller each time, or stop at a size`});s(n,r,o,i),a--};o(t,n.scope(e.lot(r)))}})}const Zt={matte:0,concrete:7,painted:1,glass:2,metal:3,emissive:4,foliage:5,water:6,wire:8,flat:13,facade:14,imagery:15,interior:16},Qt=(e=.5,t=0)=>192+Math.round(Math.max(0,Math.min(1,e))*7)*8+Math.round(Math.max(0,Math.min(1,t))*7),$t=9,en={head:10,tail:11,windows:12};var tn=class{kind=`custom`;code;spec;values;version=0;constructor(e,t){this.code=e,this.spec=t,this.values={...t.uniforms??{}}}set(e,t){if(!(e in this.values))throw new W(`E-WORLD-004`,`material has no uniform '${e}'`,{fix:`its uniforms: ${Object.keys(this.values).join(`, `)||`none`}`});return this.values[e]=t,this.version++,this}get animated(){return/\btime\b/.test(`${this.spec.vertex??``} ${this.spec.fragment}`)}};const nn=[];function rn(e,t={}){if(e===`ghost`)return{kind:e,code:128+Math.round(Math.max(0,Math.min(1,t.opacity??.35))*63)};if(e===`pbr`)return{kind:e,code:Qt(t.roughness,t.metal)};let n=Zt[e];if(n===void 0)throw new W(`E-DATA-001`,`no material '${e}'`,{fix:`one of ${[...Object.keys(Zt),`ghost`].join(`, `)}, or material.custom({ fragment })`});return{kind:e,code:n}}rn.custom=e=>{if(!e||typeof e.fragment!=`string`)throw new W(`E-SHADER-001`,`material.custom needs a fragment (WGSL statements)`,{fix:`material.custom({ fragment: 'color = vec4f(1.0, 0.0, 0.0, 1.0);' })`});if(nn.length>=64)throw new W(`E-SHADER-001`,`more than 64 custom materials in one page`,{fix:`reuse one, with uniforms for what differs`});let t=new tn(64+nn.length,e);return nn.push(t),t};const an=e=>e>=64&&e<128?nn[e-64]??null:null;function on(e){return e==null?0:typeof e==`string`?e===`ghost`?rn(`ghost`).code:e===`pbr`?Qt():Zt[e]??0:e.code}function sn(e){return e===Zt.wire?`wire`:e===9?`line`:e===17?`emit`:e>=128&&e<192?`ghost`:e>=64&&e<128?`custom`:`solid`}function cn(e,t,n){let r=Math.floor(e.length/t),i=!1;for(let t=0;t<r;t+=3)if(sn(n(e,t))!==`solid`){i=!0;break}if(!i)return null;let a=[],o=[],s=[],c=/* @__PURE__ */ new Map,l=[],u=[],d=(n,r)=>{for(let i=0;i<t;i++)n.push(e[r*t+i])};for(let t=0;t+2<r;t+=3){let r=n(e,t),i=sn(r);if(i===`solid`)d(a,t),d(a,t+1),d(a,t+2);else if(i===`line`)d(o,t),d(o,t+1);else if(i===`ghost`)d(s,t),d(s,t+1),d(s,t+2);else if(i===`emit`)d(u,t),d(u,t+1),d(u,t+2);else if(i===`custom`){let e=c.get(r);e||c.set(r,e=[]),d(e,t),d(e,t+1),d(e,t+2)}else l.push(t)}return l.length&&ln(e,t,l,o),{solid:Float32Array.from(a),lines:o.length?Float32Array.from(o):null,ghost:s.length?Float32Array.from(s):null,custom:new Map([...c].map(([e,t])=>[e,Float32Array.from(t)])),emit:u.length?Float32Array.from(u):null}}function ln(e,t,n,r){let i=n=>`${Math.round(e[n*t]*1e4)},${Math.round(e[n*t+1]*1e4)},${Math.round(e[n*t+2]*1e4)}`,a=n=>{let r=(r,i)=>e[(n+r)*t+i],i=r(1,0)-r(0,0),a=r(1,1)-r(0,1),o=r(1,2)-r(0,2),s=r(2,0)-r(0,0),c=r(2,1)-r(0,1),l=r(2,2)-r(0,2),u=a*l-o*c,d=o*s-i*l,f=i*c-a*s,p=Math.hypot(u,d,f)||1;return[u/p,d/p,f/p]},o=/* @__PURE__ */ new Map;for(let e of n){let t=a(e);for(let[n,r]of[[0,1],[1,2],[2,0]]){let a=e+n,s=e+r,c=i(a),l=i(s),u=c<l?`${c}|${l}`:`${l}|${c}`,d=o.get(u);d?(d.count++,d.n[0]*t[0]+d.n[1]*t[1]+d.n[2]*t[2]>.9995&&(d.flat=!0)):o.set(u,{a,b:s,n:t,flat:!1,count:1})}}for(let n of o.values())if(!n.flat)for(let i of[n.a,n.b])for(let n=0;n<t;n++)r.push(e[i*t+n])}const un=(e,t)=>Math.floor(e[t*8+7]/65536),dn=(e,t)=>{let n=e[t*6+5];return n>=65536?0:n%256},fn={A:`.###.#...##...#######...##...##...#`,B:`####.#...##...#####.#...##...#####.`,C:`.###.#...##....#....#....#...#.###.`,D:`####.#...##...##...##...##...#####.`,E:`######....#....####.#....#....#####`,F:`######....#....####.#....#....#....`,G:`.###.#...##....#.####...##...#.####`,H:`#...##...##...#######...##...##...#`,I:`.###...#....#....#....#....#...###.`,J:`..###...#....#....#....#.#..#..##..`,K:`#...##..#.#.#..##...#.#..#..#.#...#`,L:`#....#....#....#....#....#....#####`,M:`#...###.###.#.##...##...##...##...#`,N:`#...###..##.#.##..###...##...##...#`,O:`.###.#...##...##...##...##...#.###.`,P:`####.#...##...#####.#....#....#....`,Q:`.###.#...##...##...##.#.##..#..##.#`,R:`####.#...##...#####.#.#..#..#.#...#`,S:`.#####....#.....###.....#....#####.`,T:`#####..#....#....#....#....#....#..`,U:`#...##...##...##...##...##...#.###.`,V:`#...##...##...##...##...#.#.#...#..`,W:`#...##...##...##.#.##.#.##.#.#.#.#.`,X:`#...##...#.#.#...#...#.#.#...##...#`,Y:`#...##...#.#.#...#....#....#....#..`,Z:`#####....#...#...#...#...#....#####`,0:`.###.#...##..###.#.###..##...#.###.`,1:`..#...##....#....#....#....#...###.`,2:`.###.#...#....#...#...#...#...#####`,3:`#####...#...#.....#.....##...#.###.`,4:`...#...##..#.#.#..#.#####...#....#.`,5:`######....####.....#....##...#.###.`,6:`..##..#...#....####.#...##...#.###.`,7:`#####....#...#...#...#....#....#...`,8:`.###.#...##...#.###.#...##...#.###.`,9:`.###.#...##...#.####....#...#..##..`," ":`...................................`,"-":`................###................`,".":`...............................##..`,",":`..........................##...#...`,":":`......##...##.........##...##......`,"/":`.....#...#...#...#...#.............`,"&":`.##..#..#.#.#...#...#.#.##..#..##.#`,"!":`..#....#....#....#....#.........#..`,"?":`.###.#...#....#...#...#.........#..`,"'":`..#....#...#.......................`,"+":`.......#....#..#####..#....#.......`,"#":`.#.#..#.#.#####.#.#.#####.#.#..#.#.`,"(":`...#...#...#....#....#.....#.....#.`,")":`.#.....#.....#....#....#...#...#...`,"*":`.......#..#.#.#.###.#.#.#..#.......`,"=":`..........#####.....#####..........`},pn=/* @__PURE__ */ new Map;function mn(e){let t=e.toUpperCase(),n=pn.get(t);if(n)return n;let r=fn[t]??fn[` `];n=[];for(let e=0;e<7;e++)for(let t=0;t<5;t++){if(r[e*5+t]!==`#`)continue;let i=t;for(;i+1<5&&r[e*5+i+1]===`#`;)i++;n.push([t,e,i+1]),t=i}return pn.set(t,n),n}const hn={matte:0,painted:1,glass:2,metal:3,emissive:4,foliage:5,water:6};var gn=class extends Ht{tris=0;emitting=!1;tintColor=null;get triangles(){return this.tris}solid(e,t={}){if(!this.drawn(t))return this;let n=t.at??[0,0,0],r=t.axis??`y`,i=r===`y`?jt:r===`x`?[0,1,0,-1,0,0,0,0,1,0,0,0]:[1,0,0,0,0,1,0,-1,0,0,0,0];return this.within(Mt(Mt(Nt(n),Pt(t.turn??0)),i),()=>{let n=Math.max(3,t.sides??(this.level>0?8:16));if(e===`box`){let e=_n(t.size,1),n=Math.min(t.round??0,e[0]/2,e[1]/2,e[2]/2);n>0?this.roundedBox(e,n,t):this.box(e,t);return}let r=t.radius??(typeof t.size==`number`?t.size/2:.5),i=typeof t.size==`number`?t.size:t.size?t.size[1]:1;if(e===`sphere`)this.revolve(Array.from({length:Math.max(4,Math.round(n/2))+1},(e,t)=>{let i=-Math.PI/2+t/Math.max(4,Math.round(n/2))*Math.PI;return[Math.cos(i)*r,Math.sin(i)*r]}),n,!0,t);else if(e===`cylinder`||e===`cone`){let a=e===`cone`?t.top??0:r;this.revolve([[0,-i/2],[r,-i/2],[a,i/2],[0,i/2]],n,!1,t)}else if(e===`capsule`){let e=Math.max(3,Math.round(n/4)),a=[];for(let t=0;t<=e;t++){let n=-Math.PI/2+t/e*(Math.PI/2);a.push([Math.cos(n)*r,-i/2+r+Math.sin(n)*r])}for(let t=0;t<=e;t++){let n=t/e*(Math.PI/2);a.push([Math.cos(n)*r,i/2-r+Math.sin(n)*r])}this.revolve(a,n,!0,t)}else throw new W(`E-DATA-001`,`solid '${e}' is not one of box, cylinder, sphere, cone, capsule`,{fix:`b.solid('box', { size: [1, 2, 1] })`})}),t.label!==void 0&&this.within(Mt(Nt(n),Pt(t.turn??0)),()=>this.solidLabel(e,t)),this}label(e,t={}){if(!this.drawn(t)||!e)return this;let n=q(t.facing??[0,0,1]),r=Math.abs(n[1])>.95?[0,0,-1]:[0,1,0],i=t.along?q(t.along):q(zt(r,n)),a=q(zt(n,i));i=q(zt(a,n));let o=t.height??.3,s=o/7,c=s*6,l=t.at??[0,0,0],u=t.off??.004,d=[...e],f=-(d.length*c-s)/2,p=(e,t)=>K(K(K(l,i,e),a,t),n,u);return d.forEach((e,r)=>{for(let[i,a,l]of mn(e)){let e=f+r*c+i*s,u=f+r*c+l*s,d=o/2-a*s,m=d-s;this.tri(p(e,m),p(u,m),p(u,d),[n,n,n],t),this.tri(p(e,m),p(u,d),p(e,d),[n,n,n],t)}}),this}solidLabel(e,t){let n=typeof t.label==`string`?{text:t.label}:t.label,r=n.on??`front`,i=e===`box`?_n(t.size,1):(()=>{let n=t.radius??(typeof t.size==`number`?t.size/2:.5),r=typeof t.size==`number`?t.size:t.size?t.size[1]:1;return[n*2,e===`sphere`?n*2:r,n*2]})(),a={front:[[0,0,i[2]/2],[0,0,1],i[0],i[1]],back:[[0,0,-i[2]/2],[0,0,-1],i[0],i[1]],right:[[i[0]/2,0,0],[1,0,0],i[2],i[1]],left:[[-i[0]/2,0,0],[-1,0,0],i[2],i[1]],top:[[0,i[1]/2,0],[0,1,0],i[0],i[2]]},[o,s,c,l]=a[r]??a.front,u=Math.max(1,[...n.text].length),d=n.height??Math.min(l*.6,c*.9/(u*6/7)),f=q(zt(Math.abs(s[1])>.95?[0,0,-1]:[0,1,0],s)),p=q(zt(s,f)),m=n.at??[0,0];this.label(n.text,{at:K(K(o,f,m[0]),p,m[1]),facing:s,height:d,off:e===`box`?.004:.01,color:n.color??`#ffffff`,material:n.material,lamps:n.lamps,detail:t.detail})}extrude(e,t,n={}){return this.sweep(e,{up:t},n)}lathe(e,t={}){return this.sweep(e,{around:`y`,segments:t.segments,angle:t.angle},t)}sweep(e,t,n={}){if(!this.drawn(n))return this;let r=vn(e,this.level),i=!Array.isArray(e)||!0,a=!Array.isArray(e)&&`circle`in e;if(!Array.isArray(t)&&`around`in t)return this.revolve(r,Math.max(3,t.segments??(this.level>0?10:24)),n.smooth??!0,n,(t.angle??360)*Math.PI/180),this;let o=Array.isArray(t)?t:[[0,0,0],[0,t.up,0]];if(o.length<2)return this;let s=!Array.isArray(t),c=o.length,l=typeof n.scale==`number`?[n.scale,n.scale]:n.scale??[1,1],u=(n.twist??0)*Math.PI/180,d=[],f=0,p=[0];for(let e=1;e<c;e++)f+=Math.hypot(...G(o[e],o[e-1])),p.push(f);let m=null;for(let e=0;e<c;e++){let t=f?p[e]/f:0,n=l[0]+(l[1]-l[0])*t,i=u*t,a=q(G(o[Math.min(c-1,e+1)],o[Math.max(0,e-1)]));e>0&&e<c-1&&(a=q(K(q(G(o[e],o[e-1])),q(G(o[e+1],o[e])))));let h,g;if(s)h=[1,0,0],g=[0,0,1];else{let e=q(m?G(m,K([0,0,0],a,Bt(m,a))):zt(a,Math.abs(a[1])>.95?[1,0,0]:[0,1,0]));Number.isFinite(e[0])||(e=[1,0,0]),m=e,h=e,g=zt(a,e)}let _=Math.cos(i),v=Math.sin(i);d.push(r.map(([t,r])=>{let i=(t*_-r*v)*n,a=(t*v+r*_)*n;return K(K(o[e],h,i),g,a)}))}let h=r.length,g=i&&h>2,_=n.smooth??a,v=d.map(e=>e.reduce((t,n)=>K(t,n,1/e.length),[0,0,0]));for(let e=0;e+1<c;e++)for(let t=0;t<(g?h:h-1);t++){let r=d[e][t],i=d[e][(t+1)%h],a=d[e+1][(t+1)%h],o=d[e+1][t];if(_){let t=q(G(r,v[e])),s=q(G(i,v[e])),c=q(G(a,v[e+1])),l=q(G(o,v[e+1]));this.face4(r,i,a,o,[t,s,c,l],n,v[e])}else this.face4(r,i,a,o,null,n,v[e])}if((n.caps??g)&&g){let e=bn(r),t=d[0],i=d[c-1],a=q(G(v[c-1],v[0]));for(let r=0;r+2<e.length;r+=3)this.faceOut(t[e[r]],t[e[r+1]],t[e[r+2]],[-a[0],-a[1],-a[2]],n),this.faceOut(i[e[r]],i[e[r+1]],i[e[r+2]],a,n)}return this}copy(e,t){if(`emit`in t){let n=t.emit,r=typeof n.count==`number`?[n.count,n.count,n.count]:n.count,i=Math.min(1023,Math.max(0,Math.round(r[0]??0))),a=Math.min(1023,Math.max(0,Math.round(r[Math.min(this.level,r.length-1)]??i)));if(this.emitters.length>=2)throw new W(`E-DATA-001`,`a shape has at most 2 copies over time (copy: emit)`,{fix:`one emitter with more copies`});let o=this.emitters.length;this.emitters.push({at:this.place([0,0,0]),life:Math.max(.05,n.life??3),rise:n.rise??1,spread:n.spread??.3,grow:n.grow??1,fade:[n.fade?.[0]??1,n.fade?.[1]??0],drift:[n.drift?.[0]??0,n.drift?.[1]??0],count:i});for(let t=0;t<a;t++){this.mb.normalNow=o*1024+Math.floor(t*Math.max(i,a)/a),this.emitting=!0;try{e(this,t)}finally{this.emitting=!1,this.mb.normalNow=null}}return this}if(`mirror`in t)return t.keep!==!1&&e(this,0),this.within(It(t.mirror===`x`?[-1,1,1]:[1,1,-1]),()=>e(this,1));if(`repeat`in t){let n=t.axis===void 0?null:t.axis===`x`?[1,0,0]:t.axis===`y`?null:t.axis===`z`?[0,0,1]:q(t.axis),r=t.at??[0,0,0];for(let i=0;i<t.repeat;i++){let a=(t.turn??0)*i,o=n?Ft(n,a):Pt(a),s=t.at?Mt(Nt(r),Mt(o,Nt(Vt(r)))):o;this.within(Mt(Nt([(t.step?.[0]??0)*i,(t.step?.[1]??0)*i,(t.step?.[2]??0)*i]),s),()=>e(this,i))}return this}if(`along`in t){let n=t.along,r=0,i=[0];for(let e=1;e<n.length;e++)r+=Math.hypot(...G(n[e],n[e-1])),i.push(r);let a=Math.max(1,Math.floor(r/Math.max(1e-6,t.every))+1);for(let o=0;o<a;o++){let a=Math.min(r,o*t.every),s=1;for(;s<n.length-1&&i[s]<a;)s++;let c=(a-i[s-1])/Math.max(1e-9,i[s]-i[s-1]),l=K(n[s-1],G(n[s],n[s-1]),c),u=G(n[s],n[s-1]),d=t.align===!1?0:-Math.atan2(u[2],u[0])*180/Math.PI;this.within(Mt(Nt(l),Pt(d)),()=>e(this,o))}return this}if(`scatter`in t){let n=t.scatter,r=1/0,i=-1/0,a=1/0,o=-1/0;for(let[e,t]of n)r=Math.min(r,e),i=Math.max(i,e),a=Math.min(a,t),o=Math.max(o,t);let s=(t.seed??1)*2654435761>>>0||1,c=()=>(s^=s<<13,s^=s>>>17,s^=s<<5,s>>>=0,s/4294967296);for(let s=0,l=0;l<t.count&&s<t.count*30;s++){let s=r+(i-r)*c(),u=a+(o-a)*c();if(!yn(n,s,u))continue;let d=t.turn===!1?0:c()*360;this.within(Mt(Nt([s,0,u]),Pt(d)),()=>e(this,l)),l++}return this}throw new W(`E-DATA-001`,`copy needs how: { mirror } | { repeat } | { along, every } | { scatter, count }`,{fix:`b.copy((b, i) => b.solid('box'), { repeat: 5, step: [2, 0, 0] })`})}scope(e){if(`footprint`in e)return Jt.footprint(this,e.footprint,e.height);let t=e.size,n=e.at??[0,0,0],r=(e.turn??0)*Math.PI/180,i=[Math.cos(r),0,Math.sin(r)],a=[-Math.sin(r),0,Math.cos(r)];return new Jt(this,K(K(n,i,-t[0]/2),a,-t[2]/2),i,[0,1,0],a,[t[0],t[1],t[2]])}mesh(e,t={}){if(!this.drawn(t))return this;if(t.lines){let n=t.color??`#ffffff`;for(let t=0;t+1<e.length;t+=2){let r=this.place(e[t]),i=this.place(e[t+1]),a=[0,1,0];this.mb.solidVertex(r,a,n,9).solidVertex(i,a,n,9).solidVertex(i,a,n,9)}return this}if(!t.smooth){for(let n=0;n+2<e.length;n+=3)this.tri(e[n],e[n+1],e[n+2],null,t);return this}let n=Math.floor(e.length/3),r=[],i=[],a=/* @__PURE__ */ new Map,o=Math.cos((t.crease??50)*Math.PI/180),s=e=>`${Math.round(e[0]*1e4)},${Math.round(e[1]*1e4)},${Math.round(e[2]*1e4)}`,c=(e,t,n)=>{let r=q(G(t,e)),i=q(G(n,e));return Math.acos(Math.max(-1,Math.min(1,Bt(r,i))))};for(let t=0;t<n;t++){let n=e[t*3],o=e[t*3+1],l=e[t*3+2];r.push(q(zt(G(o,n),G(l,n)))),i.push(c(n,o,l),c(o,l,n),c(l,n,o)),[n,o,l].forEach((e,n)=>{let r=s(e),i=a.get(r);i?i.push(t*3+n):a.set(r,[t*3+n])})}for(let c=0;c<n;c++){let n=r[c];if(!(Math.abs(n[0])+Math.abs(n[1])+Math.abs(n[2])>.5))continue;let l=e=>{let t=[0,0,0];for(let c of a.get(s(e))){let e=r[Math.floor(c/3)];Bt(e,n)>=o&&(t=K(t,e,i[c]))}return q(t)},u=e[c*3],d=e[c*3+1],f=e[c*3+2];this.tri(u,d,f,[l(u),l(d),l(f)],t)}return this}drawn(e){return(e.detail??2)>=this.level}tri(e,t,n,r,i){let a=this.place(e),o=this.place(t),s=this.place(n),c=zt(G(o,a),G(s,a));if(Math.hypot(c[0],c[1],c[2])<1e-14)return;let l,u,d;r?(l=this.dir(r[0]),u=this.dir(r[1]),d=this.dir(r[2]),Rt(this.m)<0&&(l=Vt(l),u=Vt(u),d=Vt(d))):l=u=d=q(c),Bt(c,K(K(l,u),d))<0&&([o,s]=[s,o],[u,d]=[d,u]);let f=this.emitting?17:i.lamps?en[i.lamps]??0:on(i.material),p=i.tint&&this.tintColor!==null?this.tintColor:i.color??`#ffffff`,m=!!i.tint;this.mb.solidVertex(a,l,p,f,m).solidVertex(o,u,p,f,m).solidVertex(s,d,p,f,m),this.tris++}face4(e,t,n,r,i,a,o){if(i){this.tri(e,t,n,[i[0],i[1],i[2]],a),this.tri(e,n,r,[i[0],i[2],i[3]],a);return}let s=q(zt(G(t,e),G(n,e))),c=[(e[0]+n[0])/2,(e[1]+n[1])/2,(e[2]+n[2])/2];Bt(s,G(c,o))<0&&(s=Vt(s)),this.tri(e,t,n,[s,s,s],a),this.tri(e,n,r,[s,s,s],a)}faceOut(e,t,n,r,i){this.tri(e,t,n,[r,r,r],i)}box(e,t){let[n,r,i]=[e[0]/2,e[1]/2,e[2]/2],a=(e,t,a)=>[e*n,t*r,a*i],o=[[[1,0,0],[a(1,-1,-1),a(1,-1,1),a(1,1,1),a(1,1,-1)]],[[-1,0,0],[a(-1,-1,1),a(-1,-1,-1),a(-1,1,-1),a(-1,1,1)]],[[0,1,0],[a(-1,1,-1),a(1,1,-1),a(1,1,1),a(-1,1,1)]],[[0,-1,0],[a(-1,-1,1),a(1,-1,1),a(1,-1,-1),a(-1,-1,-1)]],[[0,0,1],[a(1,-1,1),a(-1,-1,1),a(-1,1,1),a(1,1,1)]],[[0,0,-1],[a(-1,-1,-1),a(1,-1,-1),a(1,1,-1),a(-1,1,-1)]]];for(let[e,n]of o)e[1]<0&&(this.level>0||t.bottom===!1)||(this.tri(n[0],n[1],n[2],[e,e,e],t),this.tri(n[0],n[2],n[3],[e,e,e],t))}roundedBox(e,t,n){let r=this.level>0?8:16,i=this.level>0?4:8,a=[e[0]/2-t,e[1]/2-t,e[2]/2-t],o=(e,n)=>{let o=e/r*Math.PI*2,s=n/i*Math.PI,c=[Math.sin(s)*Math.cos(o),Math.cos(s),Math.sin(s)*Math.sin(o)];return[[Math.sign(c[0])*a[0]+c[0]*t,Math.sign(c[1])*a[1]+c[1]*t,Math.sign(c[2])*a[2]+c[2]*t],c]};for(let e=0;e<i;e++)for(let t=0;t<r;t++){let[r,i]=o(t,e),[a,s]=o(t+1,e),[c,l]=o(t+1,e+1),[u,d]=o(t,e+1);this.tri(r,a,c,[i,s,l],n),this.tri(r,c,u,[i,l,d],n)}}revolve(e,t,n,r,i=Math.PI*2){let a=i>=Math.PI*2-1e-6?t:Math.max(1,Math.round(t*i/(Math.PI*2))),o=e.slice(0,-1).map((t,n)=>{let r=e[n+1],i=r[1]-t[1],a=r[0]-t[0],o=Math.hypot(i,a)||1;return[i/o,-a/o]}),s=(e,t)=>{let r=o[e],i=o[t];if(!n||!i||r[0]*i[0]+r[1]*i[1]<Math.cos(50*Math.PI/180))return r;let a=[r[0]+i[0],r[1]+i[1]],s=Math.hypot(a[0],a[1])||1;return[a[0]/s,a[1]/s]};for(let i=0;i+1<e.length;i++){let[c,l]=e[i],[u,d]=e[i+1];if(c<1e-9&&u<1e-9)continue;let[f,p]=o[i],[m,h]=n&&c<1e-6?[0,l<d?-1:1]:s(i,i-1),[g,_]=n&&u<1e-6?[0,d<l?-1:1]:s(i,i+1);for(let e=0;e<a;e++){let i=e/t*Math.PI*2,a=(e+1)/t*Math.PI*2,o=Math.cos(i),s=Math.sin(i),v=Math.cos(a),y=Math.sin(a),b=[c*o,l,c*s],x=[c*v,l,c*y],S=[u*v,d,u*y],C=[u*o,d,u*s],w=[m*o,h,m*s],T=[m*v,h,m*y],E=[g*v,_,g*y],D=[g*o,_,g*s];if(n)this.tri(b,x,S,[w,T,E],r),this.tri(b,S,C,[w,E,D],r);else{let e=(i+a)/2,t=[f*Math.cos(e),p,f*Math.sin(e)];this.tri(b,x,S,[t,t,t],r),this.tri(b,S,C,[t,t,t],r)}}}}};const _n=(e,t)=>typeof e==`number`?[e,e,e]:e?[e[0],e[1],e[2]]:[t,t,t];function vn(e,t){if(Array.isArray(e))return e;if(`circle`in e){let n=Math.max(3,e.sides??(t>0?6:12));return Array.from({length:n},(t,r)=>[Math.cos(r/n*Math.PI*2)*e.circle,Math.sin(r/n*Math.PI*2)*e.circle])}let[n,r]=e.rect;return[[-n/2,-r/2],[n/2,-r/2],[n/2,r/2],[-n/2,r/2]]}function yn(e,t,n){let r=!1;for(let i=0,a=e.length-1;i<e.length;a=i++){let[o,s]=e[i],[c,l]=e[a];s>n!=l>n&&t<(c-o)*(n-s)/(l-s)+o&&(r=!r)}return r}function bn(e){let t=e.length;if(t<3)return[];let n=0;for(let r=0,i=t-1;r<t;i=r++)n+=e[i][0]*e[r][1]-e[r][0]*e[i][1];let r=Array.from({length:t},(e,t)=>t);n<0&&r.reverse();let i=[],a=0,o=(e,t,n)=>(t[0]-e[0])*(n[1]-e[1])-(t[1]-e[1])*(n[0]-e[0]);for(;r.length>3&&a++<t*t;){let t=!1;for(let n=0;n<r.length;n++){let a=r[(n+r.length-1)%r.length],s=r[n],c=r[(n+1)%r.length],l=e[a],u=e[s],d=e[c];if(o(l,u,d)<=0)continue;let f=!0;for(let t of r){if(t===a||t===s||t===c)continue;let n=e[t];if(o(l,u,n)>=0&&o(u,d,n)>=0&&o(d,l,n)>=0){f=!1;break}}if(f){i.push(a,s,c),r.splice(n,1),t=!0;break}}if(!t)break}return r.length===3&&i.push(r[0],r[1],r[2]),i}const xn=`
vec3 unpackRGB(float c) {
  uint i = uint(round(c));
  return vec3(float((i >> 16u) & 255u), float((i >> 8u) & 255u), float(i & 255u)) / 255.0;
}
vec3 unpackNormal(float n) {
  uint i = uint(round(n));
  vec2 e = vec2(float(i >> 12u), float(i & 4095u)) / 4095.0 * 2.0 - vec2(1.0, 1.0);
  vec3 v = vec3(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
  float k = max(-v.z, 0.0);
  v.x = v.x + (v.x >= 0.0 ? -k : k);
  v.y = v.y + (v.y >= 0.0 ? -k : k);
  return normalize(vec3(v.x, v.z, v.y));
}
vec3 unpackCard(float c) {
  float code = round(c);
  if (code < 65536.0) {
    return vec3(0.0, 0.0, 0.0);
  }
  float kind = floor(code / 65536.0);
  float rest = code - kind * 65536.0 - 1.0;
  return vec3((rest - 256.0 * floor(rest / 256.0)) / 255.0, floor(rest / 256.0) / 255.0, kind);
}
float leafMask(vec2 uv, float kind) {
  vec2 p = uv * 2.0 - vec2(1.0, 1.0);
  float hit = 0.0;
  for (int k = 0; k < 9; k++) {
    float a = float(k) * 2.39996 + kind * 1.3;
    float r = k == 0 ? 0.0 : 0.62 * sqrt(float(k) / 8.0);
    vec2 c = vec2(cos(a), sin(a)) * r;
    vec2 dir = k == 0 ? vec2(0.0, 1.0) : normalize(c + vec2(0.0001, 0.0));
    vec2 d = p - c;
    float along = dot(d, dir);
    float across = dot(d, vec2(-dir.y, dir.x));
    float len = (kind > 1.5 && kind < 2.5) ? 0.55 : (kind > 2.5 ? 0.55 : 0.5);
    float wid = (kind > 1.5 && kind < 2.5) ? 0.04 : (kind > 2.5 ? 0.12 : 0.3);
    hit = max(hit, step((along * along) / (len * len) + (across * across) / (wid * wid), 1.0));
  }
  return hit;
}
`,Sn=`#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
precision highp sampler2DArray;
layout(std140) uniform Frame { mat4 viewProj; vec4 eye; vec4 size; vec4 sun; vec4 misc; vec4 geo; vec4 pick; vec4 region; mat4 inv; vec4 fbsize; mat4 light; vec4 sunColor; vec4 ambient; vec4 shadow; vec4 haze; vec4 weather; vec4 theme; vec4 mute; vec4 themeGround; } frame;
// aslnut writes WebGPU's clip space (depth 0..w); GL's is -w..w. The region zooms onto a few pixels for picking
vec4 clipOut(vec4 p) { return vec4(p.xy * frame.region.xy + frame.region.zw * p.w, p.z * 2.0 - p.w, p.w); }
// the ground's place in the depth order (in WebGPU's depth, before clipOut): a little farther than its own depth
const float GROUND_DEPTH_BIAS = 2e-6;
vec4 groundClip(vec4 p) { return vec4(p.xy, p.z + GROUND_DEPTH_BIAS * p.w, p.w); }
vec4 pickOut(float layer, float index) {
  uint i = uint(max(index, 0.0) + 0.5);
  return vec4(layer, float(i & 255u), float((i >> 8u) & 255u), float((i >> 16u) & 255u)) / 255.0;
}
// the analysis view's muting (core › render › style › theme): keep 0..1 is how much of it is a subject
vec3 themed(vec3 c, float keep) {
  float k = frame.mute.w * (1.0 - keep);
  if (k <= 0.0) {
    return c;
  }
  float l = dot(c, vec3(0.299, 0.587, 0.114));
  vec3 m = mix(c, vec3(l), frame.mute.x);
  m = mix(m, frame.theme.rgb * (2.0 * l), frame.theme.w);
  m = (m - vec3(0.5)) * frame.mute.z + vec3(0.5);
  m = max(m * frame.mute.y, vec3(0.0));
  return mix(c, m, k);
}
vec3 themedGround(vec3 c) {
  return themed(c, 0.0) * mix(1.0, frame.themeGround.x, frame.mute.w);
}
// core › render › drawing › instancing: one size rule for every mark and shape (s: units 0 CSS px / 1 metres, least,
// most CSS px)
vec3 viewEye() {
  if (frame.pick.y > 0.5) {
    vec4 h = frame.inv * vec4(0.0, 0.0, 1.0, 0.0);
    return h.xyz / h.w;
  }
  return frame.eye.xyz;
}
float cssMpp(vec3 p) {
  return 2.0 * length(viewEye() - p) * frame.misc.z / max(frame.fbsize.w, 1.0);
}
float sizePx(float v, vec4 s, vec3 p) {
  float px = v;
  if (s.x > 0.5) {
    px = v / max(cssMpp(p), 0.000000001);
  }
  return clamp(px, s.y, s.z);
}
float sizeKeep(float m, vec4 s, vec3 p) {
  float px = m / max(cssMpp(p), 0.000000001);
  return clamp(px, s.y, s.z) / max(px, 0.000000001);
}
vec4 packDepth(float d) {
  vec3 e = fract(vec3(1.0, 255.0, 65025.0) * clamp(d, 0.0, 0.999999));
  return vec4(e - vec3(e.y, e.z, 0.0) / 255.0, 1.0);
}
`,Cn=e=>`
uniform highp sampler2D shadowMap;
float unpackDepth(vec4 c) {
  return dot(c.rgb, vec3(1.0, 1.0 / 255.0, 1.0 / 65025.0));
}
float shadowTap(ivec2 p, float z) {
  ivec2 n = textureSize(shadowMap, 0);
  ivec2 q = clamp(p, ivec2(0, 0), n - ivec2(1, 1));
  return step(z, unpackDepth(texelFetch(shadowMap, q, 0)));
}
float sunShadow(vec3 world, vec3 n) {
  if (frame.shadow.x <= 0.0) {
    return 1.0;
  }
  vec4 p = frame.light * vec4(world + n * frame.shadow.z, 1.0);
  vec3 q = p.xyz / p.w;
  vec2 uv = vec2(q.x * 0.5 + 0.5, q.y * 0.5 + 0.5);
  if (uv.x <= 0.0 || uv.y <= 0.0 || uv.x >= 1.0 || uv.y >= 1.0 || q.z >= 1.0) {
    return 1.0;
  }
  vec2 t = uv / frame.shadow.y - vec2(0.5, 0.5);
  ivec2 i = ivec2(floor(t));
  vec2 f = fract(t);
  // the depth bias grows where the sun grazes the surface (a low sun on flat ground: no stripes of acne)
  float nl = clamp(dot(n, frame.sun.xyz), 0.05, 1.0);
  float z = q.z - frame.shadow.w - frame.fbsize.z * min(sqrt(1.0 - nl * nl) / nl, 8.0);
  float a = shadowTap(i, z);
  float b = shadowTap(i + ivec2(1, 0), z);
  float c = shadowTap(i + ivec2(0, 1), z);
  float d = shadowTap(i + ivec2(1, 1), z);
  float edge = smoothstep(0.0, 0.08, min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y)));
  return 1.0 - frame.shadow.x * edge * (1.0 - mix(mix(a, b, f.x), mix(c, d, f.x), f.y));
}
// the cloud deck's shadow on a place: the deck's pattern (the sky's, with fewer octaves) where the line to the sun
// crosses its base (frame.pick.z, m; 0: none), its place frame.geo.zw, its cover frame.weather.z; 1 unshaded
float cloudHash(vec2 q) {
  vec2 p = fract(q * vec2(0.1031, 0.103));
  p = p + dot(p, p.yx + 33.33);
  return fract((p.x + p.y) * p.x);
}
float cloudNoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 w = f * f * (3.0 - 2.0 * f);
  return mix(mix(cloudHash(i), cloudHash(i + vec2(1.0, 0.0)), w.x), mix(cloudHash(i + vec2(0.0, 1.0)), cloudHash(i + vec2(1.0, 1.0)), w.x), w.y);
}
float cloudShade(vec3 world) {
  if (frame.pick.z <= 0.0 || frame.weather.z < 0.01 || frame.sun.y < 0.03 || world.y >= frame.pick.z) {
    return 1.0;
  }
  vec2 p = (world.xz + frame.sun.xz * ((frame.pick.z - world.y) / frame.sun.y) + frame.geo.zw) / 3200.0;
  float n = 0.5 * cloudNoise(p) + 0.25 * cloudNoise(p * 2.03 + vec2(17.1, 17.1)) + 0.125 * cloudNoise(p * 4.12 + vec2(51.6, 51.6)) + 0.06;
  float c = 1.0 - frame.weather.z;
  return 1.0 - 0.75 * smoothstep(c * 0.75, c * 0.75 + 0.28, n);
}
vec3 sunLit(vec3 base, vec3 n, vec3 world) {
  float nd = max(dot(n, frame.sun.xyz), 0.0);
  float sh = 0.0;
  if (nd > 0.0) {
    sh = sunShadow(world, n) * cloudShade(world);
  }
  vec3 sky = frame.ambient.rgb * (0.62 + 0.38 * n.y) + frame.ambient.rgb * frame.ambient.w * (0.5 - 0.5 * n.y) + vec3(0.8, 0.85, 1.0) * frame.weather.y;
  float wet = frame.weather.x * max(n.y, 0.0);
  vec3 v = normalize(frame.eye.xyz - world);
  float glint = pow(max(dot(reflect(-v, n), frame.sun.xyz), 0.0), 60.0) * wet * nd * sh;
  float fres = wet * pow(1.0 - max(dot(v, n), 0.0), 4.0);
  vec3 c = base * (1.0 - 0.35 * wet) * (sky + frame.sunColor.rgb * nd * sh);
  c = mix(c, frame.haze.rgb * 0.8, fres * 0.6) + frame.sunColor.rgb * glint;
  return mix(c, vec3(0.8) + 0.2 * (vec3(1.0) - exp(-(c - vec3(0.8)) / 0.2)), vec3(greaterThan(c, vec3(0.8))));
}
vec3 airFog(vec3 c, vec3 world) {
  if (frame.haze.w <= 0.0) {
    return c;
  }
  float d = distance(world, frame.eye.xyz) / frame.haze.w;
  return mix(c, frame.haze.rgb, 1.0 - exp(-d * d));
}
`,wn=`${Sn}
layout(std140) uniform T { vec4 o; vec4 v; vec4 w; vec4 l; } tb;
struct Tile { vec4 o; vec4 v; vec4 w; vec4 l; };
Tile t;
/*@mparams*/
`,Tn=`${wn}
uniform highp sampler2DArray ground;
const int GN = 51;
const float GSTEP = 10.0;
float gLoad(int x, int z) {
  return texelFetch(ground, ivec3(clamp(x, 0, GN - 1), clamp(z, 0, GN - 1), int(t.l.w)), 0).x;
}
float groundAt(float ax, float az) {
  if (t.l.w < 0.0) {
    return 0.0;
  }
  float gx = clamp(ax / GSTEP, 0.0, float(GN - 1));
  float gz = clamp(az / GSTEP, 0.0, float(GN - 1));
  int x0 = int(floor(gx));
  int z0 = int(floor(gz));
  float fx = gx - float(x0);
  float fz = gz - float(z0);
  return mix(mix(gLoad(x0, z0), gLoad(x0 + 1, z0), fx), mix(gLoad(x0, z0 + 1), gLoad(x0 + 1, z0 + 1), fx), fz);
}
${xn}
vec3 placeOf(float x, float h, float z) {
  return vec3(t.o.x + x * t.o.z, h, t.o.y + z * t.o.w);
}
vec3 towardEye(vec3 p) {
  vec3 e = frame.eye.xyz - p;
  float d = length(e);
  return p + e / max(d, 0.001) * min(0.5 * frame.size.z * 2.0 * d * frame.misc.z / frame.size.y, d * 0.25);
}
out vec3 v_col;
out vec3 v_n;
out vec3 v_world;
flat out float v_id;
out vec3 v_leaf;
flat out vec4 v_ck;
flat out vec4 v_co;
/*@mvaryingsOut*/
/*@functions*/
`,En=`${wn}
uniform highp sampler2D tints;
uniform highp sampler2D picture;
// a surface field's colours on this mesh (solid-field.glsl.ts sets it): a theme's subject, kept
float fieldNear = 0.0;
${Cn(1)}
${xn}
/*@functions*/
float leafCut(vec3 leaf, vec3 world) {
  float kind = floor(leaf.z);
  float keep = (leaf.z - kind) / 0.45;
  float h = fract(sin(dot(floor(world * 0.8), vec3(12.9898, 78.233, 37.719))) * 43758.5453);
  return leafMask(leaf.xy, kind) * step(h, keep + 0.001);
}
float dither(vec2 p) {
  return fract(52.9829189 * fract(0.06711056 * floor(p.x) + 0.00583715 * floor(p.y)));
}
vec4 roomBehind(vec3 world) {
  vec3 d = normalize(world - frame.eye.xyz);
  vec3 size = vec3(4.0, 3.2, 4.0);
  vec3 p = world / size;
  vec3 cell = floor(p + d * 0.002);
  vec3 dd = d / size;
  vec3 safe = mix(dd, vec3(0.00001), lessThan(abs(dd), vec3(0.00001)));
  vec3 tt = (cell + step(vec3(0.0), dd) - p) / safe;
  float k = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.5453);
  vec3 col = vec3(0.78, 0.72, 0.62) * (0.8 + 0.35 * k);
  float t = min(tt.x, tt.z);
  if (tt.y < t) {
    t = tt.y;
    col = d.y < 0.0 ? vec3(0.42, 0.32, 0.24) : vec3(0.86, 0.84, 0.8);
  }
  return vec4(col / (1.0 + t * 0.5), k);
}
float facadeWindow(vec3 world, vec3 n) {
  if (abs(n.y) > 0.5) {
    return 0.0;
  }
  vec3 tg = normalize(vec3(-n.z, 0.0, n.x));
  float fu = fract(dot(world, tg) / 3.2);
  float fv = fract(world.y / 3.2);
  return step(0.22, fu) * step(fu, 0.78) * step(0.3, fv) * step(fv, 0.78);
}
vec3 pbrLit(vec3 c, vec3 n, vec3 world, float k) {
  float q = floor(k + 0.5);
  float rough = max(0.04, floor(q / 8.0) / 7.0);
  float metal = (q - floor(q / 8.0) * 8.0) / 7.0;
  vec3 v = normalize(frame.eye.xyz - world);
  vec3 h = normalize(v + frame.sun.xyz);
  float nl = max(dot(n, frame.sun.xyz), 0.0);
  float nv = max(dot(n, v), 0.001);
  float nh = max(dot(n, h), 0.0);
  float a = rough * rough;
  float dd = nh * nh * (a * a - 1.0) + 1.0;
  float dgx = a * a / (3.14159 * dd * dd);
  vec3 f0 = mix(vec3(0.04), c, metal);
  vec3 f = f0 + (vec3(1.0) - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  float kk = (rough + 1.0) * (rough + 1.0) / 8.0;
  float g = (nl / (nl * (1.0 - kk) + kk)) * (nv / (nv * (1.0 - kk) + kk));
  vec3 spec = dgx * g * f / max(4.0 * nl * nv, 0.001);
  vec3 lit = mix(c * (0.62 + 0.38 * max(dot(n, normalize(vec3(-0.45, 0.8, -0.35))), 0.0)), sunLit(c, n, world), frame.sunColor.w);
  vec3 env = mix(frame.ambient.rgb, frame.haze.rgb, 0.5) * f * (1.0 - 0.7 * rough);
  return lit * (1.0 - metal) + spec * frame.sunColor.rgb * nl * frame.sunColor.w + env;
}
vec3 metalSheet(vec3 c, vec3 n, vec3 world, vec3 lit) {
  vec3 v = normalize(frame.eye.xyz - world);
  vec3 h = normalize(v + frame.sun.xyz);
  float nl = max(dot(n, frame.sun.xyz), 0.0);
  float nv = max(dot(n, v), 0.05);
  float nh = max(dot(n, h), 0.0);
  float a = 0.55 * 0.55;
  float dd = nh * nh * (a * a - 1.0) + 1.0;
  float dgx = a * a / (3.14159 * dd * dd);
  float kk = (0.55 + 1.0) * (0.55 + 1.0) / 8.0;
  float g = (nl / (nl * (1.0 - kk) + kk)) * (nv / (nv * (1.0 - kk) + kk));
  vec3 f0 = mix(vec3(0.06), c, 0.35);
  vec3 f = f0 + (vec3(1.0) - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  float sh = 0.0;
  if (nl > 0.0) {
    sh = sunShadow(world, n) * cloudShade(world);
  }
  vec3 spec = min(dgx * g * f / (4.0 * nl * nv + 0.001) * nl * sh * frame.sunColor.w, vec3(0.45)) * frame.sunColor.rgb;
  vec3 env = mix(frame.ambient.rgb, frame.haze.rgb, 0.5) * f * 0.45;
  vec3 s = lit * (vec3(1.0) - f) + spec + env;
  return mix(s, vec3(0.8) + 0.2 * (vec3(1.0) - exp(-(s - vec3(0.8)) / 0.2)), vec3(greaterThan(s, vec3(0.8))));
}
void tileIn(vec4 ck, vec4 co) {
  t = Tile(co, vec4(tb.v.x, tb.v.y, ck.x, ck.y), vec4(tb.w.xyz, ck.z), vec4(tb.l.x, tb.l.y, ck.w, tb.l.w));
}
vec3 shadeSolid(vec3 base, vec3 n0, vec3 world, float id, float mat0) {
  int k = int(id + 0.5);
  vec4 tn = vec4(0.0);
  if (t.l.z >= 0.0) {
    tn = texelFetch(tints, ivec2(k % 256, min(int(t.l.z) + k / 256, textureSize(tints, 0).y - 1)), 0);
  }
  vec3 c = mix(base, tn.rgb, tn.a);
  float keep = step(0.002, tn.a);
  keep = max(keep, fieldNear);
  if (abs(id - t.v.z) < 0.5) {
    c = mix(c, t.w.rgb, 0.45);
    keep = 1.0;
  }
  if ((mat0 > 7.5 && mat0 < 9.5) || (mat0 > 12.5 && mat0 < 13.5)) {
    return airFog(themed(c, keep), world);
  }
  vec3 n = n0;
  if (dot(n, frame.eye.xyz - world) < 0.0) {
    n = -n;
  }
  float mat = mat0;
  if (mat0 > 13.5 && mat0 < 14.5) {
    mat = facadeWindow(world, n) > 0.5 ? 12.0 : 0.0;
  }
  if (mat0 > 15.5 && mat0 < 16.5) {
    mat = 12.0;
  }
  bool rooms = t.l.y > 0.5 || (mat0 > 15.5 && mat0 < 16.5);
  vec3 plain = c * (0.62 + 0.38 * max(dot(n, normalize(vec3(-0.45, 0.8, -0.35))), 0.0));
  vec3 o = mix(plain, sunLit(c, n, world), frame.sunColor.w);
  // Concrete diffuse sky/ground bounce, scaled by daylight.
  if (mat > 6.5 && mat < 7.5) {
    o = max(o, c * 0.60 * frame.sun.w);
  }
  if ((mat > 0.5 && mat < 6.5) || (mat > 11.5 && mat < 12.5)) {
    vec3 v = normalize(frame.eye.xyz - world);
    float fres = pow(1.0 - max(dot(v, n), 0.0), 4.0);
    vec3 h = normalize(v + frame.sun.xyz);
    float lit = frame.sunColor.w * max(dot(n, frame.sun.xyz), 0.0);
    float night = 1.0 - clamp(frame.sun.w * 4.0, 0.0, 1.0);
    if ((mat > 1.5 && mat < 2.5) || mat > 11.5) {
      o = mix(c * 0.35 * (frame.ambient.rgb + frame.sunColor.rgb * 0.3), frame.haze.rgb, 0.25 + 0.6 * fres) + frame.sunColor.rgb * pow(max(dot(n, h), 0.0), 90.0) * lit;
    } else if (mat > 2.5 && mat < 3.5) {
      o = metalSheet(c, n, world, o);
    } else if (mat > 3.5 && mat < 4.5) {
      o = c * (0.7 + 0.6 * night);
    } else if (mat > 4.5 && mat < 5.5) {
      o = o + c * frame.sunColor.rgb * 0.3 * max(dot(-n, frame.sun.xyz), 0.0) * frame.sunColor.w;
    } else {
      o = o + frame.sunColor.rgb * pow(max(dot(n, h), 0.0), 24.0) * lit * 0.3 + frame.haze.rgb * fres * 0.15;
    }
  }
  if (mat0 > 14.5 && mat0 < 15.5) {
    ivec2 pd = textureSize(picture, 0);
    if (pd.x > 1) {
      float lx = (world.x - t.o.x) / t.o.z / 500.0;
      float lz = (world.z - t.o.y) / t.o.w / 500.0;
      vec3 im = texelFetch(picture, ivec2(clamp(vec2(lx, lz), vec2(0.0), vec2(0.999)) * vec2(pd)), 0).rgb;
      o = mix(im * (0.62 + 0.38 * max(dot(n, normalize(vec3(-0.45, 0.8, -0.35))), 0.0)), sunLit(im, n, world), frame.sunColor.w);
    }
  }
  if (mat0 > 191.5) {
    o = pbrLit(c, n, world, mat0 - 192.0);
  }
  float wet = frame.weather.x;
  if (wet > 0.0 && (mat < 0.5 || (mat > 6.5 && mat < 7.5) || mat0 > 191.5)) {
    vec3 wv = normalize(frame.eye.xyz - world);
    vec3 wh = normalize(wv + frame.sun.xyz);
    float wl = frame.sunColor.w * max(dot(n, frame.sun.xyz), 0.0);
    o = o * (1.0 - 0.3 * wet) + frame.sunColor.rgb * pow(max(dot(n, wh), 0.0), 70.0) * wl * 0.6 * wet + frame.haze.rgb * pow(1.0 - max(dot(wv, n), 0.0), 4.0) * 0.3 * wet;
  }
  if (mat > 9.5 && mat < 12.5) {
    float dark = 1.0 - clamp(frame.sun.w * 4.0, 0.0, 1.0);
    if (mat < 11.5) {
      o = mix(o * 0.85 + c * 0.12, c * (mat < 10.5 ? 1.6 : 1.35), dark);
    } else {
      vec4 inner = roomBehind(world);
      bool deep = rooms;
      float r = deep ? inner.w : fract(sin(dot(floor(world * vec3(0.33, 0.36, 0.33)), vec3(12.9898, 78.233, 37.719))) * 43758.5453);
      float room = step(1.0 - t.l.x, r);
      vec3 warm = deep ? inner.rgb * vec3(1.3, 1.05, 0.75) : vec3(1.0, 0.8, 0.52) * (0.75 + 0.25 * fract(room * 7.31 + world.y));
      o = mix(o, inner.rgb * (frame.ambient.rgb + frame.sunColor.rgb * 0.25), deep ? 0.55 * (1.0 - dark) : 0.0);
      o = mix(o, warm, room * dark);
    }
  }
  return airFog(themed(o, keep), world);
}
`,Dn=`${En}
in vec3 v_col;
in vec3 v_n;
in vec3 v_world;
flat in float v_id;
in vec3 v_leaf;
flat in vec4 v_ck;
flat in vec4 v_co;
/*@mvaryingsIn*/
out vec4 outColor;
void main() {
  tileIn(v_ck, v_co);
  /*@fieldfragment*/
  if (t.v.w < 1.0 && dither(gl_FragCoord.xy) > t.v.w) {
    discard;
  }
  if (v_leaf.z > 0.5 && leafCut(v_leaf, v_world) < 0.5) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    outColor = packDepth(gl_FragCoord.z);
    return;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(t.v.y, t.w.w * 65536.0 + v_id);
    return;
  }
  float m = max(-v_leaf.z, 0.0);
  vec4 color = vec4(shadeSolid(v_col, normalize(v_n), v_world, v_id, m), m > 127.5 && m < 191.5 ? clamp((m - 128.0) / 63.0, 0.0, 1.0) : 1.0);
  /*@mfinputs*/
  /*@mfragment*/
  outColor = color;
}
`,On={vertex:`${Tn}
layout(location = 0) in vec4 a;
layout(location = 1) in vec4 b;
void main() {
  t = Tile(tb.o, tb.v, tb.w, tb.l);
  float h = groundAt(b.x, b.y) * t.v.x + a.y;
  vec3 position = placeOf(a.x, h, a.z);
  v_col = unpackRGB(a.w);
  /*@fieldvertex*/
  v_n = unpackNormal(b.z);
  float mat = floor(b.w / 65536.0);
  v_id = b.w - mat * 65536.0;
  v_leaf = vec3(0.0, 0.0, -mat);
  v_ck = vec4(t.v.z, t.v.w, t.w.w, t.l.z);
  v_co = t.o;
  /*@mvinputs*/
  /*@mvertex*/
  /*@mvoutputs*/
  gl_Position = clipOut(frame.viewProj * vec4(towardEye(position), 1.0));
  v_world = position;
}
`,fragment:Dn},kn=`
layout(std140) uniform S { vec4 a; vec4 b; vec4 c; vec4 d; vec4 j[32]; vec4 e[6]; } si;
uniform highp sampler2D recs;
vec4 recAt(float r, int k) {
  int a = int(r + 0.5) + k;
  int w = textureSize(recs, 0).x;
  return texelFetch(recs, ivec2(a % w, a / w), 0);
}
void tileAt(float slot) {
  int s = int(slot + 0.5) * 2;
  vec4 a = texelFetch(recs, ivec2(s, 0), 0);
  vec4 b = texelFetch(recs, ivec2(s + 1, 0), 0);
  t = Tile(a, vec4(tb.v.x, tb.v.y, b.x, b.y), vec4(tb.w.xyz, slot), vec4(tb.l.x, tb.l.y, b.w, b.z));
}
float keepOf(float scale, vec3 p) {
  return si.a.y > 0.5 ? sizeKeep(si.a.x * scale, vec4(1.0, si.a.z, si.a.w, 0.0), p) : 1.0;
}
vec3 rotAxis(vec3 p, vec3 a, float t) {
  float c = cos(t);
  return p * c + cross(a, p) * sin(t) + a * dot(a, p) * (1.0 - c);
}
float jointFormula(int k, float time, float seed) {
  /*@jointFormula*/
  return 0.0;
}
float jointValue(int k, float r, float seed) {
  vec4 d = si.j[k * 4 + 2];
  vec4 s = si.j[k * 4 + 3];
  float v = 0.0;
  if (d.x == 1.0) {
    float rate = d.y;
    float phase = seed * 6.2831853;
    if (s.y >= 0.0) {
      vec4 q = recAt(r, int(s.y + 0.5) / 4);
      bool even = (int(s.y + 0.5) % 4) == 0;
      rate = even ? q.x : q.z;
      phase = even ? q.y : q.w;
    }
    v = phase + rate * (frame.eye.w - si.c.x) + s.z;
  } else if (d.x == 2.0) {
    v = jointFormula(k, frame.eye.w, seed) * (si.j[k * 4].w == 3.0 ? 1.0 : 0.017453293);
  }
  if (d.z < d.w) {
    v = clamp(v, d.z, d.w);
  }
  return v;
}
vec4 partPivot(int part) {
  if (part == 0) {
    return vec4(0.0);
  }
  for (int k = 0; k < int(si.c.y); k++) {
    if (int(si.j[k * 4 + 3].x + 0.5) == part) {
      return vec4(si.j[k * 4].xyz, 1.0);
    }
  }
  return vec4(0.0);
}
void jointMove(vec3 q, vec3 n, int part, float r, float seed, out vec3 mq, out vec3 mn) {
  mq = q;
  mn = n;
  if (part == 0) {
    return;
  }
  for (int k = 0; k < int(si.c.y); k++) {
    if (int(si.j[k * 4 + 3].x + 0.5) != part) {
      continue;
    }
    vec4 pv = si.j[k * 4];
    vec3 ax = si.j[k * 4 + 1].xyz;
    float v = jointValue(k, r, seed);
    if (pv.w == 3.0) {
      mq = mq + ax * v;
    } else {
      mq = pv.xyz + rotAxis(mq - pv.xyz, ax, v);
      mn = rotAxis(mn, ax, v);
    }
  }
}
`,An={vertex:`${Tn}${kn}
layout(location = 0) in float r;
layout(location = 3) in vec4 v;
layout(location = 4) in float vc;
layout(location = 5) in float card;
void main() {
  vec4 r0 = recAt(r, 0);
  vec4 r1 = recAt(r, 1);
  vec4 r2 = recAt(r, 2);
  tileAt(r2.w);
  bool leafy = card >= 65536.0;
  int ci = leafy ? 0 : int(card + 0.5);
  int lo = ci % 512;
  int piece = ci / 512;
  vec3 mq; vec3 mn;
  jointMove(v.xyz, unpackNormal(v.w), piece % 16, r, r2.z, mq, mn);
  float c = cos(r1.w);
  float s = sin(r1.w);
  float g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  float keep = keepOf(max(r1.x, r1.z), placeOf(r0.x, g + si.a.x * 0.5 * r1.y, r0.y));
  vec4 pp = partPivot(piece % 16);
  vec3 q = (pp.w > 0.5 ? pp.xyz * r1.xyz + (mq - pp.xyz) * r1.x : mq * r1.xyz) * keep;
  vec3 rr = vec3(q.x * c - q.z * s, q.y, q.x * s + q.z * c);
  float h = g + rr.y;
  vec3 position = placeOf(r0.x + rr.x, h, r0.y + rr.z);
  vec3 n = pp.w > 0.5 ? mn : mn / r1.xyz;
  vec3 tint = unpackRGB(r2.x);
  bool own = !leafy && lo >= 256;
  int grp = piece / 16;
  if (si.b.w > 0.5 && grp > 0 && grp <= 4) {
    float gt = recAt(r, 3)[grp - 1];
    if (gt >= 0.0) {
      tint = unpackRGB(gt);
      own = false;
    }
  }
  v_col = (own ? vec3(1.0) : tint) * unpackRGB(vc);
  float code = leafy ? card : float(lo % 256);
  v_n = vec3(n.x * c - n.z * s, n.y, n.x * s + n.z * c);
  v_id = r0.w;
  v_leaf = leafy ? unpackCard(card) : vec3(0.0, 0.0, -code);
  if (v_leaf.z > 0.5) {
    v_leaf.z = v_leaf.z + 0.45 * clamp(1.0 - r2.y, 0.0, 1.0);
  }
  v_ck = vec4(t.v.z, t.v.w, t.w.w, t.l.z);
  v_co = t.o;
  /*@mvinputs*/
  /*@mvertex*/
  /*@mvoutputs*/
  gl_Position = clipOut(frame.viewProj * vec4(towardEye(position), 1.0));
  v_world = position;
}
`,fragment:Dn},jn=`#version 300 es
precision highp float;
precision highp int;
layout(std140) uniform B { mat4 view; vec4 cell; vec4 mode; } b;
${xn}
`,Mn={vertex:`${jn}
layout(location = 3) in vec4 v;
layout(location = 4) in float vc;
layout(location = 5) in float card;
out vec3 v_col;
out vec3 v_n;
out vec3 v_leaf;
out vec2 v_q;
void main() {
  vec4 p = b.view * vec4(v.xyz, 1.0);
  v_q = p.xy;
  vec2 xy = p.xy * b.cell.z + b.cell.xy;
  gl_Position = vec4(xy.x, -xy.y, -p.z, 1.0);
  v_col = unpackRGB(vc);
  v_n = unpackNormal(v.w);
  v_leaf = unpackCard(card);
}
`,fragment:`${jn}
in vec3 v_col;
in vec3 v_n;
in vec3 v_leaf;
in vec2 v_q;
out vec4 outColor;
void main() {
  if (abs(v_q.x) > 1.0 || abs(v_q.y) > 1.0) {
    discard;
  }
  if (v_leaf.z > 0.5 && leafMask(v_leaf.xy, v_leaf.z) < 0.5) {
    discard;
  }
  vec3 n = normalize(v_n);
  if (dot(n, b.mode.yzw) < 0.0) {
    n = -n;
  }
  outColor = b.mode.x > 0.5 ? vec4(n * 0.5 + vec3(0.5, 0.5, 0.5), 1.0) : vec4(v_col, 1.0);
}
`},Nn=`
layout(std140) uniform IM { vec4 a; } im;
`,Pn={vertex:`${Tn}${kn}${Nn}
layout(location = 0) in float r;
out vec2 v_uv;
flat out float v_turn;
vec2 hemiOct(vec3 d) {
  vec3 a = vec3(d.x, max(d.y, 0.0), d.z) / (abs(d.x) + max(d.y, 0.0) + abs(d.z) + 0.000001);
  return vec2(a.x + a.z, a.x - a.z) * 0.5 + vec2(0.5, 0.5);
}
vec3 hemiOctDir(vec2 e) {
  vec2 f = e * 2.0 - vec2(1.0, 1.0);
  float x = (f.x + f.y) * 0.5;
  float z = (f.x - f.y) * 0.5;
  return normalize(vec3(x, max(1.0 - abs(x) - abs(z), 0.0), z));
}
void main() {
  vec4 r0 = recAt(r, 0);
  vec4 r1 = recAt(r, 1);
  vec4 r2 = recAt(r, 2);
  tileAt(r2.w);
  float g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  float s = r1.y * keepOf(max(r1.x, r1.z), placeOf(r0.x, g + im.a.y * r1.y, r0.y));
  float h = g + im.a.y * s;
  vec3 c = placeOf(r0.x, h, r0.y);
  vec3 d = normalize(frame.eye.xyz - c);
  if (frame.pick.y > 0.5) {
    d = normalize(frame.sun.xyz);
  }
  float co = cos(r1.w);
  float si = sin(r1.w);
  float n = im.a.z;
  vec2 cell = clamp(floor(hemiOct(vec3(d.x * co + d.z * si, d.y, -d.x * si + d.z * co)) * n), vec2(0.0, 0.0), vec2(n - 1.0, n - 1.0));
  vec3 bd = hemiOctDir((cell + vec2(0.5, 0.5)) / n);
  vec3 dw = vec3(bd.x * co - bd.z * si, bd.y, bd.x * si + bd.z * co);
  vec3 right = cross(vec3(0.0, 1.0, 0.0), dw);
  if (length(right) < 0.001) {
    right = vec3(co, 0.0, si);
  }
  right = normalize(right);
  vec3 up = cross(dw, right);
  int k = gl_VertexID - (gl_VertexID / 6) * 6;
  float cx = (k == 1 || k == 2 || k == 4) ? 1.0 : -1.0;
  float cy = (k == 2 || k == 4 || k == 5) ? 1.0 : -1.0;
  vec3 p = c + (right * cx + up * cy) * im.a.x * s;
  gl_Position = clipOut(frame.viewProj * vec4(p, 1.0));
  v_col = unpackRGB(r2.x);
  v_world = p;
  v_id = r0.w;
  v_leaf = vec3(0.0, 0.0, 0.0);
  v_n = vec3(0.0, 1.0, 0.0);
  v_uv = (cell + vec2(cx * 0.5 + 0.5, 0.5 - cy * 0.5)) / n;
  v_turn = r1.w;
  v_ck = vec4(t.v.z, t.v.w, t.w.w, t.l.z);
  v_co = t.o;
}
`,fragment:`${En}${Nn}
uniform highp sampler2D atlas;
uniform highp sampler2D normals;
in vec3 v_col;
in vec3 v_world;
flat in float v_id;
in vec2 v_uv;
flat in float v_turn;
flat in vec4 v_ck;
flat in vec4 v_co;
out vec4 outColor;
void main() {
  tileIn(v_ck, v_co);
  if (t.v.w < 1.0 && dither(gl_FragCoord.xy) > t.v.w) {
    discard;
  }
  ivec2 px = ivec2(clamp(v_uv * im.a.w, vec2(0.0, 0.0), vec2(im.a.w - 1.0, im.a.w - 1.0)));
  vec4 a = texelFetch(atlas, px, 0);
  if (a.a < 0.5) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    outColor = packDepth(gl_FragCoord.z);
    return;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(t.v.y, t.w.w * 65536.0 + v_id);
    return;
  }
  vec3 nl = texelFetch(normals, px, 0).xyz * 2.0 - vec3(1.0, 1.0, 1.0);
  float co = cos(v_turn);
  float si = sin(v_turn);
  vec3 nw = normalize(vec3(nl.x * co - nl.z * si, nl.y, nl.x * si + nl.z * co));
  outColor = vec4(shadeSolid(a.rgb * v_col, nw, v_world, v_id, 0.0), 1.0);
}
`},Fn=`
fn unpackRGB(c: f32) -> vec3f {
  let i = u32(round(c));
  return vec3f(f32((i >> 16u) & 255u), f32((i >> 8u) & 255u), f32(i & 255u)) / 255.0;
}
// octahedral normal (world axes: x east, y up, z south)
fn unpackNormal(n: f32) -> vec3f {
  let i = u32(round(n));
  let e = vec2f(f32(i >> 12u), f32(i & 4095u)) / 4095.0 * 2.0 - vec2f(1.0, 1.0);
  var v = vec3f(e.x, e.y, 1.0 - abs(e.x) - abs(e.y));
  let k = max(-v.z, 0.0);
  v.x = v.x + select(k, -k, v.x >= 0.0);
  v.y = v.y + select(k, -k, v.y >= 0.0);
  return normalize(vec3f(v.x, v.z, v.y));
}
// a leaf card's code (0: solid) → (u, v, kind): 1 + u + v · 256 + kind · 65536, u and v 0 or 255 at the corners
fn unpackCard(c: f32) -> vec3f {
  let code = round(c);
  if (code < 65536.0) {
    return vec3f(0.0, 0.0, 0.0);
  }
  let kind = floor(code / 65536.0);
  let rest = code - kind * 65536.0 - 1.0;
  return vec3f((rest - 256.0 * floor(rest / 256.0)) / 255.0, floor(rest / 256.0) / 255.0, kind);
}
// a cluster of leaves on a card: 1 inside a leaf, 0 between them (kind 1 broad, 2 needles, 3 narrow hanging leaves)
fn leafMask(uv: vec2f, kind: f32) -> f32 {
  let p = uv * 2.0 - vec2f(1.0, 1.0);
  var hit = 0.0;
  for (var k = 0; k < 9; k++) {
    let a = f32(k) * 2.39996 + kind * 1.3;
    let r = select(0.62 * sqrt(f32(k) / 8.0), 0.0, k == 0);
    let c = vec2f(cos(a), sin(a)) * r;
    let dir = select(normalize(c + vec2f(0.0001, 0.0)), vec2f(0.0, 1.0), k == 0);
    let d = p - c;
    let along = dot(d, dir);
    let across = dot(d, vec2f(-dir.y, dir.x));
    let len = select(select(0.5, 0.55, kind > 2.5), 0.55, kind > 1.5 && kind < 2.5);
    let wid = select(select(0.3, 0.12, kind > 2.5), 0.04, kind > 1.5 && kind < 2.5);
    hit = max(hit, step((along * along) / (len * len) + (across * across) / (wid * wid), 1.0));
  }
  return hit;
}
`,In=`
struct Frame { viewProj: mat4x4f, eye: vec4f, size: vec4f, sun: vec4f, misc: vec4f, geo: vec4f, pick: vec4f, region: vec4f, inv: mat4x4f, fbsize: vec4f, light: mat4x4f, sunColor: vec4f, ambient: vec4f, shadow: vec4f, haze: vec4f, weather: vec4f, theme: vec4f, mute: vec4f, themeGround: vec4f }
@group(0) @binding(0) var<uniform> frame: Frame;
// WebGPU's clip space already is the one aslnut writes (depth 0..w); the region zooms onto a few pixels for picking
fn clipOut(p: vec4f) -> vec4f { return vec4f(p.xy * frame.region.xy + frame.region.zw * p.w, p.z, p.w); }
// the ground's place in the depth order: a little farther than its own depth (in the depth buffer's units: some 32
// steps of a 24-bit buffer, centimetres in the world), so whatever lies on it wins every tie: a road, a verge, a park,
// paint. In the shader, not the pipeline: the same on WebGPU and WebGL2, every depth format
const GROUND_DEPTH_BIAS: f32 = 2e-6;
fn groundClip(p: vec4f) -> vec4f { return vec4f(p.xy, p.z + GROUND_DEPTH_BIAS * p.w, p.w); }
// a layer number (1..255) and a feature index (0..2^24 - 1) as a colour: what picking reads back
fn pickOut(layer: f32, index: f32) -> vec4f {
  let i = u32(max(index, 0.0) + 0.5);
  return vec4f(layer, f32(i & 255u), f32((i >> 8u) & 255u), f32((i >> 16u) & 255u)) / 255.0;
}
// core › render › style › theme: what is not an analysis view's subject muted (greyed, towards the tint keeping its
// light and dark, contrast round the middle grey, brightness), by mute.w; keep 0..1: how much of it is a subject (the
// selected, the hovered, a thing an analysis tints). No theme (mute.w 0): the colour as it is
fn themed(c: vec3f, keep: f32) -> vec3f {
  let k = frame.mute.w * (1.0 - keep);
  if (k <= 0.0) {
    return c;
  }
  let l = dot(c, vec3f(0.299, 0.587, 0.114));
  var m = mix(c, vec3f(l), frame.mute.x);
  m = mix(m, frame.theme.rgb * (2.0 * l), frame.theme.w);
  m = (m - vec3f(0.5)) * frame.mute.z + vec3f(0.5);
  m = max(m * frame.mute.y, vec3f(0.0));
  return mix(c, m, k);
}
// the ground under a theme: muted, and darker or lighter than what stands on it (themeGround.x)
fn themedGround(c: vec3f) -> vec3f {
  return themed(c, 0.0) * mix(1.0, frame.themeGround.x, frame.mute.w);
}
// core › render › drawing › instancing: one size rule for every mark and shape. s: (units: 0 CSS px, 1 metres; the
// least and the most CSS px). Metres become the CSS px they cover where p is seen (a zoom writes nothing); either is
// kept within [least, most]
// the view's own eye: in the shadow map's pass the eye is the light's, the view's found from its inverse (so a thing
// casts the shadow of the size it is drawn at)
fn viewEye() -> vec3f {
  if (frame.pick.y > 0.5) {
    let h = frame.inv * vec4f(0.0, 0.0, 1.0, 0.0);
    return h.xyz / h.w;
  }
  return frame.eye.xyz;
}
// metres per CSS px at p, as the view sees it (fbsize.w: the view's CSS height, in every pass)
fn cssMpp(p: vec3f) -> f32 {
  return 2.0 * length(viewEye() - p) * frame.misc.z / max(frame.fbsize.w, 1.0);
}
fn sizePx(v: f32, s: vec4f, p: vec3f) -> f32 {
  var px = v;
  if (s.x > 0.5) {
    px = v / max(cssMpp(p), 0.000000001);
  }
  return clamp(px, s.y, s.z);
}
// the share of its own size a thing of m metres across keeps at p so that it covers [least, most] CSS px (1: as it is)
fn sizeKeep(m: f32, s: vec4f, p: vec3f) -> f32 {
  let px = m / max(cssMpp(p), 0.000000001);
  return clamp(px, s.y, s.z) / max(px, 0.000000001);
}
// a depth 0..1 in three bytes (the shadow map is an 8-bit colour target on every chip)
fn packDepth(d: f32) -> vec4f {
  let e = fract(vec3f(1.0, 255.0, 65025.0) * clamp(d, 0.0, 0.999999));
  return vec4f(e - vec3f(e.y, e.z, 0.0) / 255.0, 1.0);
}
`,Ln=e=>`
@group(1) @binding(${e}) var shadowMap: texture_2d<f32>;
fn unpackDepth(c: vec4f) -> f32 {
  return dot(c.rgb, vec3f(1.0, 1.0 / 255.0, 1.0 / 65025.0));
}
// 1 where the stored depth is beyond z (lit), 0 where something nearer the sun is there
fn shadowTap(p: vec2i, z: f32) -> f32 {
  let n = vec2i(textureDimensions(shadowMap, 0));
  let q = clamp(p, vec2i(0, 0), n - vec2i(1, 1));
  return step(z, unpackDepth(textureLoad(shadowMap, q, 0)));
}
// how much of the sun reaches a place: 0 in shadow … 1; 1 outside the shadow map. n: the surface normal (the place
// is moved along it by frame.shadow.z metres before the look-up: no acne on the faces that cast)
fn sunShadow(world: vec3f, n: vec3f) -> f32 {
  if (frame.shadow.x <= 0.0) {
    return 1.0;
  }
  let p = frame.light * vec4f(world + n * frame.shadow.z, 1.0);
  let q = p.xyz / p.w;
  let uv = vec2f(q.x * 0.5 + 0.5, 0.5 - q.y * 0.5);
  if (uv.x <= 0.0 || uv.y <= 0.0 || uv.x >= 1.0 || uv.y >= 1.0 || q.z >= 1.0) {
    return 1.0;
  }
  let t = uv / frame.shadow.y - vec2f(0.5, 0.5);
  let i = vec2i(floor(t));
  let f = fract(t);
  // the depth bias grows where the sun grazes the surface (a low sun on flat ground: no stripes of acne)
  let nl = clamp(dot(n, frame.sun.xyz), 0.05, 1.0);
  let z = q.z - frame.shadow.w - frame.fbsize.z * min(sqrt(1.0 - nl * nl) / nl, 8.0);
  let a = shadowTap(i, z);
  let b = shadowTap(i + vec2i(1, 0), z);
  let c = shadowTap(i + vec2i(0, 1), z);
  let d = shadowTap(i + vec2i(1, 1), z);
  // fading out towards the shadow map's edge (no line where the shadows stop)
  let edge = smoothstep(0.0, 0.08, min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y)));
  return 1.0 - frame.shadow.x * edge * (1.0 - mix(mix(a, b, f.x), mix(c, d, f.x), f.y));
}
// the cloud deck's shadow on a place: the deck's pattern (the sky's, with fewer octaves) where the line to the sun
// crosses its base (frame.pick.z, m; 0: none), its place frame.geo.zw, its cover frame.weather.z; 1 unshaded
fn cloudHash(q: vec2f) -> f32 {
  var p = fract(q * vec2f(0.1031, 0.103));
  p = p + dot(p, p.yx + 33.33);
  return fract((p.x + p.y) * p.x);
}
fn cloudNoise(p: vec2f) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(cloudHash(i), cloudHash(i + vec2f(1.0, 0.0)), w.x), mix(cloudHash(i + vec2f(0.0, 1.0)), cloudHash(i + vec2f(1.0, 1.0)), w.x), w.y);
}
fn cloudShade(world: vec3f) -> f32 {
  if (frame.pick.z <= 0.0 || frame.weather.z < 0.01 || frame.sun.y < 0.03 || world.y >= frame.pick.z) {
    return 1.0;
  }
  let p = (world.xz + frame.sun.xz * ((frame.pick.z - world.y) / frame.sun.y) + frame.geo.zw) / 3200.0;
  let n = 0.5 * cloudNoise(p) + 0.25 * cloudNoise(p * 2.03 + vec2f(17.1, 17.1)) + 0.125 * cloudNoise(p * 4.12 + vec2f(51.6, 51.6)) + 0.06;
  let c = 1.0 - frame.weather.z;
  return 1.0 - 0.75 * smoothstep(c * 0.75, c * 0.75 + 0.28, n);
}
// a colour under the sky's light and the sun's (with its shadow); the frame's light is off (sunColor.w 0): unchanged
fn sunLit(base: vec3f, n: vec3f, world: vec3f) -> vec3f {
  let nd = max(dot(n, frame.sun.xyz), 0.0);
  var sh = 0.0;
  if (nd > 0.0) {
    sh = sunShadow(world, n) * cloudShade(world);
  }
  let sky = frame.ambient.rgb * (0.62 + 0.38 * n.y) + frame.ambient.rgb * frame.ambient.w * (0.5 - 0.5 * n.y) + vec3f(0.8, 0.85, 1.0) * frame.weather.y;
  // wet: darker, and what faces up gleams with the sky and the sun
  let wet = frame.weather.x * max(n.y, 0.0);
  let v = normalize(frame.eye.xyz - world);
  let glint = pow(max(dot(reflect(-v, n), frame.sun.xyz), 0.0), 60.0) * wet * nd * sh;
  let fres = wet * pow(1.0 - max(dot(v, n), 0.0), 4.0);
  var c = base * (1.0 - 0.35 * wet) * (sky + frame.sunColor.rgb * nd * sh);
  c = mix(c, frame.haze.rgb * 0.8, fres * 0.6) + frame.sunColor.rgb * glint;
  // a soft shoulder over 0.8: a wall in a low sun glows warm instead of clipping to white
  return select(c, vec3f(0.8) + 0.2 * (vec3f(1.0) - exp(-(c - vec3f(0.8)) / 0.2)), c > vec3f(0.8));
}
// the air between the eye and a place: fog and rain (how far one sees, frame.haze.w) blend it into the haze
fn airFog(c: vec3f, world: vec3f) -> vec3f {
  if (frame.haze.w <= 0.0) {
    return c;
  }
  let d = distance(world, frame.eye.xyz) / frame.haze.w;
  return mix(c, frame.haze.rgb, 1.0 - exp(-d * d));
}
`,Rn=`${In}
// o: (the tile's corner in world metres x, z; metres per tile metre x, z) · v: (exaggeration, pick number, hovered
// id, shown 0..1) · w: (highlight rgb, the tile's slot: its part of the picking index) · l: (the share of windows lit at
// night, rooms behind them, the first row of its tints, its ground's layer)
struct T { o: vec4f, v: vec4f, w: vec4f, l: vec4f }
@group(0) @binding(1) var<uniform> tb: T;
// the tile drawn now: a mesh's chunk's block; an instance's chunk's, from its layer's records (core › geometry › shapes ›
// instances: every chunk's clones of a form in one draw), its layer's block for what all its chunks share
var<private> t: T;
/*@mparams*/
/*@functions*/
@group(1) @binding(0) var ground: texture_2d_array<f32>;
// a colour per id over the solid's own (rgb, how much): an analysis tints houses by their walk to a park
@group(1) @binding(2) var tints: texture_2d<f32>;
// the chunk's square seen from above (the imagery material, 15: its 500 m, x east, z south; 1 × 1: none)
@group(1) @binding(3) var picture: texture_2d<f32>;
// a surface field's colours on this mesh (core › simulation › fields › surface: solid-field.wgsl.ts sets it): a theme's
// subject, kept
var<private> fieldNear: f32;
${Ln(1)}
const GN = 51;
const GSTEP = 10.0;
fn gLoad(x: i32, z: i32) -> f32 {
  return textureLoad(ground, vec2i(clamp(x, 0, GN - 1), clamp(z, 0, GN - 1)), i32(t.l.w), 0).x;
}
// the ground under a place of the tile (true metres), bilinear in its 10 m grid
fn groundAt(ax: f32, az: f32) -> f32 {
  // (a chunk on flat ground has no grid: its ground's layer −1)
  if (t.l.w < 0.0) {
    return 0.0;
  }
  let gx = clamp(ax / GSTEP, 0.0, f32(GN - 1));
  let gz = clamp(az / GSTEP, 0.0, f32(GN - 1));
  let x0 = i32(floor(gx));
  let z0 = i32(floor(gz));
  let fx = gx - f32(x0);
  let fz = gz - f32(z0);
  return mix(mix(gLoad(x0, z0), gLoad(x0 + 1, z0), fx), mix(gLoad(x0, z0 + 1), gLoad(x0 + 1, z0 + 1), fx), fz);
}
${Fn}
// a tile's place → world metres from the view's centre (y up)
fn placeOf(x: f32, h: f32, z: f32) -> vec3f {
  return vec3f(t.o.x + x * t.o.z, h, t.o.y + z * t.o.w);
}
// a little towards the eye (half a pixel's worth): ribbons on the ground never sink into it
fn towardEye(p: vec3f) -> vec3f {
  let e = frame.eye.xyz - p;
  let d = length(e);
  return p + e / max(d, 0.001) * min(0.5 * frame.size.z * 2.0 * d * frame.misc.z / frame.size.y, d * 0.25);
}
// a card's leaves: its cut-out, thinned to the share its instance keeps (kind + 0.45 × kept)
fn leafCut(leaf: vec3f, world: vec3f) -> f32 {
  let kind = floor(leaf.z);
  let keep = (leaf.z - kind) / 0.45;
  let h = fract(sin(dot(floor(world * 0.8), vec3f(12.9898, 78.233, 37.719))) * 43758.5453);
  return leafMask(leaf.xy, kind) * step(h, keep + 0.001);
}
// a dither 0..1 per pixel (interleaved gradient noise): a tile appears by dissolving in, with no blending (depth stays
// exact)
fn dither(p: vec2f) -> f32 {
  return fract(52.9829189 * fract(0.06711056 * floor(p.x) + 0.00583715 * floor(p.y)));
}
// the room behind a window (interior mapping): rooms 4 × 3.2 × 4 m on the world's axes, the eye's ray through the glass
// into its room, the surface it meets there (a floor, a ceiling, a wall: rgb) and the room's number (w)
fn roomBehind(world: vec3f) -> vec4f {
  let d = normalize(world - frame.eye.xyz);
  let size = vec3f(4.0, 3.2, 4.0);
  let p = world / size;
  let cell = floor(p + d * 0.002);
  let dd = d / size;
  let safe = select(dd, vec3f(0.00001), abs(dd) < vec3f(0.00001));
  let tt = (cell + step(vec3f(0.0), dd) - p) / safe;
  let k = fract(sin(dot(cell, vec3f(12.9898, 78.233, 37.719))) * 43758.5453);
  var col = vec3f(0.78, 0.72, 0.62) * (0.8 + 0.35 * k);
  var t = min(tt.x, tt.z);
  if (tt.y < t) {
    t = tt.y;
    col = select(vec3f(0.86, 0.84, 0.8), vec3f(0.42, 0.32, 0.24), d.y < 0.0);
  }
  return vec4f(col / (1.0 + t * 0.5), k);
}
// a facade's windows: on walls (not roofs), a floor (3.2 m) and a bay (3.2 m) apart, a pane in the middle of each
fn facadeWindow(world: vec3f, n: vec3f) -> f32 {
  if (abs(n.y) > 0.5) {
    return 0.0;
  }
  let tg = normalize(vec3f(-n.z, 0.0, n.x));
  let fu = fract(dot(world, tg) / 3.2);
  let fv = fract(world.y / 3.2);
  return step(0.22, fu) * step(fu, 0.78) * step(0.3, fv) * step(fv, 0.78);
}
// a physically based surface (192…255: roughness × 8 + metal, each 0…7): the sun's light through a GGX highlight and
// Schlick's Fresnel, the sky's in it, the diffuse share lit as the solids are (shadows, ambient)
fn pbrLit(c: vec3f, n: vec3f, world: vec3f, k: f32) -> vec3f {
  // (the code arrives interpolated: whole again before it is split)
  let q = floor(k + 0.5);
  let rough = max(0.04, floor(q / 8.0) / 7.0);
  let metal = (q - floor(q / 8.0) * 8.0) / 7.0;
  let v = normalize(frame.eye.xyz - world);
  let h = normalize(v + frame.sun.xyz);
  let nl = max(dot(n, frame.sun.xyz), 0.0);
  let nv = max(dot(n, v), 0.001);
  let nh = max(dot(n, h), 0.0);
  let a = rough * rough;
  let dd = nh * nh * (a * a - 1.0) + 1.0;
  let dgx = a * a / (3.14159 * dd * dd);
  let f0 = mix(vec3f(0.04), c, metal);
  let f = f0 + (vec3f(1.0) - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  let kk = (rough + 1.0) * (rough + 1.0) / 8.0;
  let g = (nl / (nl * (1.0 - kk) + kk)) * (nv / (nv * (1.0 - kk) + kk));
  let spec = dgx * g * f / max(4.0 * nl * nv, 0.001);
  let lit = mix(c * (0.62 + 0.38 * max(dot(n, normalize(vec3f(-0.45, 0.8, -0.35))), 0.0)), sunLit(c, n, world), frame.sunColor.w);
  let env = mix(frame.ambient.rgb, frame.haze.rgb, 0.5) * f * (1.0 - 0.7 * rough);
  return lit * (1.0 - metal) + spec * frame.sunColor.rgb * nl * frame.sunColor.w + env;
}
// sheet metal (3: a corrugated iron roof, painted steel): the sun through a GGX highlight at roughness .55 (the
// corrugations spread it), Schlick's Fresnel from a tinted F0, shadowed as the rest; the diffuse what the Fresnel
// leaves (energy kept), the sky in it; the highlight clamped and the sum under sunLit's shoulder: a face in the sun's
// mirror angle gleams, never burns to white
fn metalSheet(c: vec3f, n: vec3f, world: vec3f, lit: vec3f) -> vec3f {
  let v = normalize(frame.eye.xyz - world);
  let h = normalize(v + frame.sun.xyz);
  let nl = max(dot(n, frame.sun.xyz), 0.0);
  let nv = max(dot(n, v), 0.05);
  let nh = max(dot(n, h), 0.0);
  let a = 0.55 * 0.55;
  let dd = nh * nh * (a * a - 1.0) + 1.0;
  let dgx = a * a / (3.14159 * dd * dd);
  let kk = (0.55 + 1.0) * (0.55 + 1.0) / 8.0;
  let g = (nl / (nl * (1.0 - kk) + kk)) * (nv / (nv * (1.0 - kk) + kk));
  let f0 = mix(vec3f(0.06), c, 0.35);
  let f = f0 + (vec3f(1.0) - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  var sh = 0.0;
  if (nl > 0.0) {
    sh = sunShadow(world, n) * cloudShade(world);
  }
  let spec = min(dgx * g * f / (4.0 * nl * nv + 0.001) * nl * sh * frame.sunColor.w, vec3f(0.45)) * frame.sunColor.rgb;
  let env = mix(frame.ambient.rgb, frame.haze.rgb, 0.5) * f * 0.45;
  let s = lit * (vec3f(1.0) - f) + spec + env;
  return select(s, vec3f(0.8) + 0.2 * (vec3f(1.0) - exp(-(s - vec3f(0.8)) / 0.2)), s > vec3f(0.8));
}
// the tile in the fragment stage: what is its chunk's own came from the vertex (ck: hovered id, shown, slot, first
// tints row; co: its corner and scales), the rest is its block's
fn tileIn(ck: vec4f, co: vec4f) {
  t = T(co, vec4f(tb.v.x, tb.v.y, ck.x, ck.y), vec4f(tb.w.xyz, ck.z), vec4f(tb.l.x, tb.l.y, ck.w, tb.l.w));
}
// the colour of a solid's fragment: tinted, selected, hovered, lit (the world's sun, or a fixed light), both sides of thin
// faces; muted by a theme unless it is a subject
fn shadeSolid(base: vec3f, n0: vec3f, world: vec3f, id: f32, mat0: f32) -> vec3f {
  let k = i32(id + 0.5);
  // (its chunk's rows in the layer's tints, from its first; none: −1)
  var tn = vec4f(0.0);
  if (t.l.z >= 0.0) {
    tn = textureLoad(tints, vec2i(k % 256, min(i32(t.l.z) + k / 256, i32(textureDimensions(tints, 0).y) - 1)), 0);
  }
  var c = mix(base, tn.rgb, tn.a);
  // a theme's subjects keep their colour: what an analysis tints (the selected too: its colour is a tint), the hovered
  var keep = step(0.002, tn.a);
  // (a surface field's colours on it: kept too)
  keep = max(keep, fieldNear);
  if (abs(id - t.v.z) < 0.5) {
    c = mix(c, t.w.rgb, 0.45);
    keep = 1.0;
  }
  // lines (a wire's edges, raw segments) and flat (13): their own colour, unlit, through the air
  if ((mat0 > 7.5 && mat0 < 9.5) || (mat0 > 12.5 && mat0 < 13.5)) {
    return airFog(themed(c, keep), world);
  }
  var n = n0;
  if (dot(n, frame.eye.xyz - world) < 0.0) {
    n = -n;
  }
  // a facade (14): its wall, and windows a floor and a bay apart (lit windows); interior (16): windows with rooms
  var mat = mat0;
  if (mat0 > 13.5 && mat0 < 14.5) {
    mat = select(0.0, 12.0, facadeWindow(world, n) > 0.5);
  }
  if (mat0 > 15.5 && mat0 < 16.5) {
    mat = 12.0;
  }
  let rooms = t.l.y > 0.5 || (mat0 > 15.5 && mat0 < 16.5);
  let plain = c * (0.62 + 0.38 * max(dot(n, normalize(vec3f(-0.45, 0.8, -0.35))), 0.0));
  var o = mix(plain, sunLit(c, n, world), frame.sunColor.w);
  // Concrete retains diffuse sky/ground bounce under a deck. Only sunlight is shadowed.
  // Daylight scales the floor, so night remains dark.
  if (mat > 6.5 && mat < 7.5) {
    o = max(o, c * 0.60 * frame.sun.w);
  }
  // its material: 1 painted, 2 glass (the sky in it), 3 metal, 4 glowing, 5 leaves (the
  // sun through them), 6 water, 10…12 lamps (a ghost, 128…191, and your own, 64…127, are lit plain)
  if ((mat > 0.5 && mat < 6.5) || (mat > 11.5 && mat < 12.5)) {
    let v = normalize(frame.eye.xyz - world);
    let fres = pow(1.0 - max(dot(v, n), 0.0), 4.0);
    let h = normalize(v + frame.sun.xyz);
    let lit = frame.sunColor.w * max(dot(n, frame.sun.xyz), 0.0);
    let night = 1.0 - clamp(frame.sun.w * 4.0, 0.0, 1.0);
    if ((mat > 1.5 && mat < 2.5) || mat > 11.5) {
      o = mix(c * 0.35 * (frame.ambient.rgb + frame.sunColor.rgb * 0.3), frame.haze.rgb, 0.25 + 0.6 * fres) + frame.sunColor.rgb * pow(max(dot(n, h), 0.0), 90.0) * lit;
    } else if (mat > 2.5 && mat < 3.5) {
      o = metalSheet(c, n, world, o);
    } else if (mat > 3.5 && mat < 4.5) {
      o = c * (0.7 + 0.6 * night);
    } else if (mat > 4.5 && mat < 5.5) {
      o = o + c * frame.sunColor.rgb * 0.3 * max(dot(-n, frame.sun.xyz), 0.0) * frame.sunColor.w;
    } else {
      o = o + frame.sunColor.rgb * pow(max(dot(n, h), 0.0), 24.0) * lit * 0.3 + frame.haze.rgb * fres * 0.15;
    }
  }
  // the imagery under it, from above (15): its chunk's picture where it has one
  if (mat0 > 14.5 && mat0 < 15.5) {
    let pd = textureDimensions(picture, 0);
    if (pd.x > 1u) {
      let lx = (world.x - t.o.x) / t.o.z / 500.0;
      let lz = (world.z - t.o.y) / t.o.w / 500.0;
      let im = textureLoad(picture, vec2i(clamp(vec2f(lx, lz), vec2f(0.0), vec2f(0.999)) * vec2f(pd)), 0).rgb;
      o = mix(im * (0.62 + 0.38 * max(dot(n, normalize(vec3f(-0.45, 0.8, -0.35))), 0.0)), sunLit(im, n, world), frame.sunColor.w);
    }
  }
  // physically based (192…255)
  if (mat0 > 191.5) {
    o = pbrLit(c, n, world, mat0 - 192.0);
  }
  // wet (the world's rain): matte, concrete and physically based surfaces darker and glossier
  let wet = frame.weather.x;
  if (wet > 0.0 && (mat < 0.5 || (mat > 6.5 && mat < 7.5) || mat0 > 191.5)) {
    let wv = normalize(frame.eye.xyz - world);
    let wh = normalize(wv + frame.sun.xyz);
    let wl = frame.sunColor.w * max(dot(n, frame.sun.xyz), 0.0);
    o = o * (1.0 - 0.3 * wet) + frame.sunColor.rgb * pow(max(dot(n, wh), 0.0), 70.0) * wl * 0.6 * wet + frame.haze.rgb * pow(1.0 - max(dot(wv, n), 0.0), 4.0) * 0.3 * wet;
  }
  // lamps (10 head, 11 tail, 12 windows): from dusk they glow (a lamp's own colour; windows: glass with a share of
  // them lit rooms, the layer's); by day a lens, a pane of glass
  if (mat > 9.5 && mat < 12.5) {
    let dark = 1.0 - clamp(frame.sun.w * 4.0, 0.0, 1.0);
    if (mat < 11.5) {
      o = mix(o * 0.85 + c * 0.12, c * select(1.35, 1.6, mat < 10.5), dark);
    } else {
      // rooms behind the glass (the layer's interior): seen by day through it, lit or dark at night
      let inner = roomBehind(world);
      let deep = rooms;
      let r = select(fract(sin(dot(floor(world * vec3f(0.33, 0.36, 0.33)), vec3f(12.9898, 78.233, 37.719))) * 43758.5453), inner.w, deep);
      let room = step(1.0 - t.l.x, r);
      let warm = select(vec3f(1.0, 0.8, 0.52) * (0.75 + 0.25 * fract(room * 7.31 + world.y)), inner.rgb * vec3f(1.3, 1.05, 0.75), deep);
      o = mix(o, inner.rgb * (frame.ambient.rgb + frame.sunColor.rgb * 0.25), select(0.0, 0.55 * (1.0 - dark), deep));
      o = mix(o, warm, room * dark);
    }
  }
  return airFog(themed(o, keep), world);
}
`,zn=`${Rn}
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec3f,
  @location(1) n: vec3f,
  @location(2) world: vec3f,
  @location(3) @interpolate(flat) id: f32,
  @location(4) leaf: vec3f,
  @location(5) @interpolate(flat) ck: vec4f,
  @location(6) @interpolate(flat) co: vec4f,
  /*@mvaryings*/
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  tileIn(i.ck, i.co);
  /*@fieldfragment*/
  if (t.v.w < 1.0 && dither(i.pos.xy) > t.v.w) {
    discard;
  }
  // a leaf card: only its leaves (in the frame, the picking and the sun's shadow map alike); a thinner crown (its
  // instance's share of leaves kept, in the kind's fraction) drops patches of them
  if (i.leaf.z > 0.5 && leafCut(i.leaf, i.world) < 0.5) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    return packDepth(i.pos.z);
  }
  if (frame.pick.x > 0.5) {
    return pickOut(t.v.y, t.w.w * 65536.0 + i.id);
  }
  let m = max(-i.leaf.z, 0.0);
  // a ghost: see-through by its code (128…191: opacity 0…1)
  var color = vec4f(shadeSolid(i.col, normalize(i.n), i.world, i.id, m), select(1.0, clamp((m - 128.0) / 63.0, 0.0, 1.0), m > 127.5 && m < 191.5));
  /*@mfinputs*/
  /*@mfragment*/
  return color;
}
`,Bn=`${zn}
struct I { @location(0) a: vec4f, @location(1) b: vec4f }
@vertex fn vs(i: I, @builtin(vertex_index) vi: u32) -> O {
  t = tb;
  let h = groundAt(i.b.x, i.b.y) * t.v.x + i.a.y;
  var position = placeOf(i.a.x, h, i.a.z);
  var o: O;
  o.col = unpackRGB(i.a.w);
  /*@fieldvertex*/
  o.n = unpackNormal(i.b.z);
  // the id's high part is the material
  let mat = floor(i.b.w / 65536.0);
  o.id = i.b.w - mat * 65536.0;
  o.leaf = vec3f(0.0, 0.0, -mat);
  o.ck = vec4f(t.v.z, t.v.w, t.w.w, t.l.z);
  o.co = t.o;
  /*@mvinputs*/
  /*@mvertex*/
  /*@mvoutputs*/
  o.pos = clipOut(frame.viewProj * vec4f(towardEye(position), 1.0));
  o.world = position;
  return o;
}
`,Vn=`
// an instance set's block (S, binding 2: one for its form's clones in every chunk): a (its shape's size across m, 1 when
// it keeps a range on screen, the least and the most CSS px: core › render › drawing › size) · b (0, texels a record, 0,
// 1 when its clones tint its groups) · c (its clock's start s, its joints, 0, 0) · d (the share of its copies over
// time the governor keeps, 0, 0, 0) · e (each emitter 3: its place, its copies' life s · rise, spread m, grow ×, its
// copies at the whole level · opacity at birth, at death, drift x, z m: core › geometry › shapes › builder › copy) · j (each joint 4:
// pivot xyz and kind (1 hinge, 2 wheel, 3 slider) · axis xyz, radius · its driver (0 none, 1 a rate, 2 a formula), the
// rate (rad or m a second), least, most (none: least ≥ most) · its part, the float of a record its clone's rate starts
// at (−1: the set's rate, a phase by the clone's seed), the set's offset (what a change of rate keeps), 0)
struct SI { a: vec4f, b: vec4f, c: vec4f, d: vec4f, j: array<vec4f, 32>, e: array<vec4f, 6> }
@group(0) @binding(2) var<uniform> si: SI;
@group(1) @binding(4) var recs: texture_2d<f32>;
// texel k of the record at texel r of the layer's records
fn recAt(r: f32, k: i32) -> vec4f {
  let a = i32(r + 0.5) + k;
  let w = i32(textureDimensions(recs, 0).x);
  return textureLoad(recs, vec2i(a % w, a / w), 0);
}
// the tile of the chunk in slot \`slot\` (the records' first row), with what its layer's block holds for every chunk
fn tileAt(slot: f32) {
  let s = i32(slot + 0.5) * 2;
  let a = textureLoad(recs, vec2i(s, 0), 0);
  let b = textureLoad(recs, vec2i(s + 1, 0), 0);
  t = T(a, vec4f(tb.v.x, tb.v.y, b.x, b.y), vec4f(tb.w.xyz, slot), vec4f(tb.l.x, tb.l.y, b.w, b.z));
}
// the share of its own size an instance of this scale keeps at p: within its set's range on screen (1: none set)
fn keepOf(scale: f32, p: vec3f) -> f32 {
  return select(1.0, sizeKeep(si.a.x * scale, vec4f(1.0, si.a.z, si.a.w, 0.0), p), si.a.y > 0.5);
}
// core › entities › kinematics: p turned by t radians round the unit axis a (Rodrigues)
fn rotAxis(p: vec3f, a: vec3f, t: f32) -> vec3f {
  let c = cos(t);
  return p * c + cross(a, p) * sin(t) + a * dot(a, p) * (1.0 - c);
}
// your formulas (a joint's value of the clock and the clone's seed; degrees, or metres for a slider)
fn jointFormula(k: i32, time: f32, seed: f32) -> f32 {
  /*@jointFormula*/
  return 0.0;
}
// a joint's value now (radians; a slider's metres): its driver's (a rate from the set's clock, its clone's own rate
// and phase, or your formula), within its limits
fn jointValue(k: i32, r: f32, seed: f32) -> f32 {
  let d = si.j[k * 4 + 2];
  let s = si.j[k * 4 + 3];
  var v = 0.0;
  if (d.x == 1.0) {
    var rate = d.y;
    var phase = seed * 6.2831853;
    if (s.y >= 0.0) {
      let q = recAt(r, i32(s.y + 0.5) / 4);
      let even = (i32(s.y + 0.5) % 4) == 0;
      rate = select(q.z, q.x, even);
      phase = select(q.w, q.y, even);
    }
    v = phase + rate * (frame.eye.w - si.c.x) + s.z;
  } else if (d.x == 2.0) {
    v = jointFormula(k, frame.eye.w, seed) * select(0.017453293, 1.0, si.j[k * 4].w == 3.0);
  }
  if (d.z < d.w) {
    v = clamp(v, d.z, d.w);
  }
  return v;
}
// the pivot of the first joint that moves part \`part\` (w: 1; 0: none moves it)
fn partPivot(part: i32) -> vec4f {
  if (part == 0) {
    return vec4f(0.0);
  }
  for (var k = 0; k < i32(si.c.y); k++) {
    if (i32(si.j[k * 4 + 3].x + 0.5) == part) {
      return vec4f(si.j[k * 4].xyz, 1.0);
    }
  }
  return vec4f(0.0);
}
// a vertex (q, its normal n) of part \`part\` moved by that part's joints: turned round the axis through the pivot (a
// hinge, a wheel), or slid along it
struct Moved { q: vec3f, n: vec3f }
fn jointMove(q: vec3f, n: vec3f, part: i32, r: f32, seed: f32) -> Moved {
  var m = Moved(q, n);
  if (part == 0) {
    return m;
  }
  for (var k = 0; k < i32(si.c.y); k++) {
    if (i32(si.j[k * 4 + 3].x + 0.5) != part) {
      continue;
    }
    let pv = si.j[k * 4];
    let ax = si.j[k * 4 + 1].xyz;
    let v = jointValue(k, r, seed);
    if (pv.w == 3.0) {
      m.q = m.q + ax * v;
    } else {
      m.q = pv.xyz + rotAxis(m.q - pv.xyz, ax, v);
      m.n = rotAxis(m.n, ax, v);
    }
  }
  return m;
}
`,Hn=`${zn}${Vn}
struct I { @location(0) r: f32, @location(3) v: vec4f, @location(4) vc: f32, @location(5) card: f32 }
@vertex fn vs(i: I) -> O {
  let r0 = recAt(i.r, 0);
  let r1 = recAt(i.r, 1);
  let r2 = recAt(i.r, 2);
  tileAt(r2.w);
  // its code: a leaf card's, or its material, its own colour (256) and its piece (part + 16 × group) × 512
  let leafy = i.card >= 65536.0;
  let ci = select(i32(i.card + 0.5), 0, leafy);
  let lo = ci % 512;
  let piece = ci / 512;
  let mv = jointMove(i.v.xyz, unpackNormal(i.v.w), piece % 16, i.r, r2.z);
  let c = cos(r1.w);
  let s = sin(r1.w);
  let g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  // its set's range on screen: the share of its size it keeps where it stands (seen at its middle)
  let keep = keepOf(max(r1.x, r1.z), placeOf(r0.x, g + si.a.x * 0.5 * r1.y, r0.y));
  // its scale and stretch (a taller tower) on what stands still; a part a joint moves rides along, its pivot stretched
  // and itself only scaled (a rotor stays round)
  let pp = partPivot(piece % 16);
  let q = select(mv.q * r1.xyz, pp.xyz * r1.xyz + (mv.q - pp.xyz) * r1.x, pp.w > 0.5) * keep;
  let rr = vec3f(q.x * c - q.z * s, q.y, q.x * s + q.z * c);
  let h = g + rr.y;
  var position = placeOf(r0.x + rr.x, h, r0.y + rr.z);
  let n = select(mv.n / r1.xyz, mv.n, pp.w > 0.5);
  var o: O;
  // 256…511: a piece that keeps its own colour (its shape has tinted pieces: only they take the clone's); a group its
  // set tints takes its clone's colour for it
  var tint = unpackRGB(r2.x);
  var own = !leafy && lo >= 256;
  let grp = piece / 16;
  if (si.b.w > 0.5 && grp > 0 && grp <= 4) {
    let gt = recAt(i.r, 3)[grp - 1];
    if (gt >= 0.0) {
      tint = unpackRGB(gt);
      own = false;
    }
  }
  o.col = select(tint, vec3f(1.0, 1.0, 1.0), own) * unpackRGB(i.vc);
  let code = select(f32(lo % 256), i.card, leafy);
  o.n = vec3f(n.x * c - n.z * s, n.y, n.x * s + n.z * c);
  o.id = r0.w;
  // a card's code, or (below 65536) its material
  o.leaf = select(vec3f(0.0, 0.0, -code), unpackCard(i.card), leafy);
  if (o.leaf.z > 0.5) {
    o.leaf.z = o.leaf.z + 0.45 * clamp(1.0 - r2.y, 0.0, 1.0);
  }
  o.ck = vec4f(t.v.z, t.v.w, t.w.w, t.l.z);
  o.co = t.o;
  /*@mvinputs*/
  /*@mvertex*/
  /*@mvoutputs*/
  o.pos = clipOut(frame.viewProj * vec4f(towardEye(position), 1.0));
  o.world = position;
  return o;
}
`,Un=`
struct B { view: mat4x4f, cell: vec4f, mode: vec4f }
@group(0) @binding(0) var<uniform> b: B;
${Fn}
struct VI { @location(3) v: vec4f, @location(4) vc: f32, @location(5) card: f32 }
struct VO { @builtin(position) pos: vec4f, @location(0) col: vec3f, @location(1) n: vec3f, @location(2) leaf: vec3f, @location(3) q: vec2f }
@vertex fn vs(i: VI) -> VO {
  let p = b.view * vec4f(i.v.xyz, 1.0);
  var o: VO;
  o.q = p.xy;
  o.pos = vec4f(p.xy * b.cell.z + b.cell.xy, 0.5 - p.z * 0.5, 1.0);
  o.col = unpackRGB(i.vc);
  o.n = unpackNormal(i.v.w);
  o.leaf = unpackCard(i.card);
  return o;
}
@fragment fn fs(o: VO) -> @location(0) vec4f {
  if (abs(o.q.x) > 1.0 || abs(o.q.y) > 1.0) {
    discard;
  }
  if (o.leaf.z > 0.5 && leafMask(o.leaf.xy, o.leaf.z) < 0.5) {
    discard;
  }
  var n = normalize(o.n);
  if (dot(n, b.mode.yzw) < 0.0) {
    n = -n;
  }
  return select(vec4f(o.col, 1.0), vec4f(n * 0.5 + vec3f(0.5, 0.5, 0.5), 1.0), b.mode.x > 0.5);
}
`,Wn=`${Rn}${Vn}
struct IM { a: vec4f }
@group(0) @binding(3) var<uniform> im: IM;
@group(1) @binding(5) var atlas: texture_2d<f32>;
@group(1) @binding(6) var normals: texture_2d<f32>;
// a direction over the upper half of a sphere → 0..1 on a square (hemi-octahedral)
fn hemiOct(d: vec3f) -> vec2f {
  let a = vec3f(d.x, max(d.y, 0.0), d.z) / (abs(d.x) + max(d.y, 0.0) + abs(d.z) + 0.000001);
  return vec2f(a.x + a.z, a.x - a.z) * 0.5 + vec2f(0.5, 0.5);
}
// the view a cell of the grid was baked from (its direction)
fn hemiOctDir(e: vec2f) -> vec3f {
  let f = e * 2.0 - vec2f(1.0, 1.0);
  let x = (f.x + f.y) * 0.5;
  let z = (f.x - f.y) * 0.5;
  return normalize(vec3f(x, max(1.0 - abs(x) - abs(z), 0.0), z));
}
struct I { @location(0) r: f32 }
struct OI {
  @builtin(position) pos: vec4f,
  @location(0) col: vec3f,
  @location(1) world: vec3f,
  @location(2) @interpolate(flat) id: f32,
  @location(3) uv: vec2f,
  @location(4) @interpolate(flat) turn: f32,
  @location(5) @interpolate(flat) ck: vec4f,
  @location(6) @interpolate(flat) co: vec4f,
}
@vertex fn vs(@builtin(vertex_index) vi: u32, i: I) -> OI {
  let r0 = recAt(i.r, 0);
  let r1 = recAt(i.r, 1);
  let r2 = recAt(i.r, 2);
  tileAt(r2.w);
  let g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  // its set's range on screen (the whole shape's, seen at its middle)
  let s = r1.y * keepOf(max(r1.x, r1.z), placeOf(r0.x, g + im.a.y * r1.y, r0.y));
  let h = g + im.a.y * s;
  let c = placeOf(r0.x, h, r0.y);
  // seen from the eye; in the sun's shadow pass, from the sun
  var d = normalize(frame.eye.xyz - c);
  if (frame.pick.y > 0.5) {
    d = normalize(frame.sun.xyz);
  }
  let co = cos(r1.w);
  let si = sin(r1.w);
  let n = im.a.z;
  let cell = clamp(floor(hemiOct(vec3f(d.x * co + d.z * si, d.y, -d.x * si + d.z * co)) * n), vec2f(0.0, 0.0), vec2f(n - 1.0, n - 1.0));
  // the card faces the view the cell was baked from, turned as the instance is
  let bd = hemiOctDir((cell + vec2f(0.5, 0.5)) / n);
  let dw = vec3f(bd.x * co - bd.z * si, bd.y, bd.x * si + bd.z * co);
  var right = cross(vec3f(0.0, 1.0, 0.0), dw);
  if (length(right) < 0.001) {
    right = vec3f(co, 0.0, si);
  }
  right = normalize(right);
  let up = cross(dw, right);
  let k = vi % 6u;
  let cx = select(-1.0, 1.0, k == 1u || k == 2u || k == 4u);
  let cy = select(-1.0, 1.0, k == 2u || k == 4u || k == 5u);
  let p = c + (right * cx + up * cy) * im.a.x * s;
  var o: OI;
  o.pos = clipOut(frame.viewProj * vec4f(p, 1.0));
  o.col = unpackRGB(r2.x);
  o.world = p;
  o.id = r0.w;
  o.uv = (cell + vec2f(cx * 0.5 + 0.5, 0.5 - cy * 0.5)) / n;
  o.turn = r1.w;
  o.ck = vec4f(t.v.z, t.v.w, t.w.w, t.l.z);
  o.co = t.o;
  return o;
}
@fragment fn fs(o: OI) -> @location(0) vec4f {
  tileIn(o.ck, o.co);
  if (t.v.w < 1.0 && dither(o.pos.xy) > t.v.w) {
    discard;
  }
  let px = vec2i(clamp(o.uv * im.a.w, vec2f(0.0, 0.0), vec2f(im.a.w - 1.0, im.a.w - 1.0)));
  let a = textureLoad(atlas, px, 0);
  if (a.a < 0.5) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    return packDepth(o.pos.z);
  }
  if (frame.pick.x > 0.5) {
    return pickOut(t.v.y, t.w.w * 65536.0 + o.id);
  }
  let nl = textureLoad(normals, px, 0).xyz * 2.0 - vec3f(1.0, 1.0, 1.0);
  let co = cos(o.turn);
  let si = sin(o.turn);
  let nw = normalize(vec3f(nl.x * co - nl.z * si, nl.y, nl.x * si + nl.z * co));
  return vec4f(shadeSolid(a.rgb * o.col, nw, o.world, o.id, 0.0), 1.0);
}
`;function Gn(e,t={}){let n=[1/0,1/0,1/0],r=[-1/0,-1/0,-1/0];for(let t=0;t+6<=e.length;t+=6)for(let i=0;i<3;i++)n[i]=Math.min(n[i],e[t+i]),r[i]=Math.max(r[i],e[t+i]);Number.isFinite(n[0])||(n=[0,0,0],r=[1,1,1]);let i=[(n[0]+r[0])/2,(n[1]+r[1])/2,(n[2]+r[2])/2],a=0;for(let t=0;t+6<=e.length;t+=6)a=Math.max(a,Math.hypot(e[t],e[t+1]-i[1],e[t+2]));let o=Math.max(2,Math.min(16,Math.round(t.views??8)));return{kind:`impostor`,shape:e,views:o,size:Math.max(o*16,Math.min(4096,t.size??o*64)),centreY:i[1],radius:a*1.02}}const Kn=e=>!!e&&e.kind===`impostor`;function qn(e,t){let n=e*2-1,r=t*2-1,i=(n+r)/2,a=(n-r)/2,o=Math.max(0,1-Math.abs(i)-Math.abs(a)),s=Math.hypot(i,o,a)||1;return[i/s,o/s,a/s]}let Jn=/* @__PURE__ */ new WeakMap;async function Yn(e,t,n){let r=Jn.get(e);r||(r=e.program({label:`impostor bake`,wgsl:Un,glsl:Mn,blocks:[`B`],vertex:{stride:6,attributes:[{name:`v`,location:3,size:4,offset:0},{name:`vc`,location:4,size:1,offset:4},{name:`card`,location:5,size:1,offset:5}]},depth:{write:!0,test:`less`},cull:`none`}),Jn.set(e,r)),await r.ready;let i=t.views,a=t.radius,o=t.centreY,s=t.shape.length/6,c=[],l=[],u=[];for(let t=0;t<i;t++)for(let d=0;d<i;d++){let f=qn((d+.5)/i,(t+.5)/i),p=f[2],m=-f[0],h=Math.hypot(p,m);h<.001?(p=1,m=0):(p/=h,m/=h);let g=[p,0,m],_=[f[1]*g[2]-f[2]*g[1],f[2]*g[0]-f[0]*g[2],f[0]*g[1]-f[1]*g[0]],v=/* @__PURE__ */ new Float32Array(16),y=[g,_,f];for(let e=0;e<3;e++){let t=y[e];v[e]=t[0]/a,v[4+e]=t[1]/a,v[8+e]=t[2]/a,v[12+e]=-(t[1]*o)/a}v[15]=1;let b=/* @__PURE__ */ new Float32Array(24);b.set(v,0),b.set([-1+(2*d+1)/i,1-(2*t+1)/i,1/i,0],16),b.set([0,f[0],f[1],f[2]],20),u.push(b);let x=e.buffer(`uniform`,b,`impostor bake cell`);c.push(x),l.push({program:r,blocks:[x],vertices:n,count:s})}let d=e.target(t.size,t.size,{sampled:!0,label:`impostor colours`}),f=e.target(t.size,t.size,{sampled:!0,label:`impostor normals`});return e.frame(l,{clear:[0,0,0,0],target:d}),u.forEach((e,t)=>{e[20]=1,c[t].write(e)}),e.frame(l,{clear:[.5,1,.5,0],target:f}),{color:d.texture,normals:f.texture,targets:[d,f],buffers:c}}function Xn(e,t){let[n,r]=Zn(e),i=0,a=0,o=0,s=0,c=0,l=0;for(let t=0;t+6<=e.length;t+=6){let n=e[t+4];i+=n>>16&255,a+=n>>8&255,o+=n&255,s++}for(let t=0;t+18<=e.length;t+=18){let n=e[t+6]-e[t],r=e[t+7]-e[t+1],i=e[t+8]-e[t+2],a=e[t+12]-e[t],o=e[t+13]-e[t+1],s=e[t+14]-e[t+2],u=Math.hypot(r*s-i*o,i*a-n*s,n*o-r*a);l+=u,e[t+5]>=256&&e[t+5]<65536&&(c+=u)}let u=s?[i/s/255,a/s/255,o/s/255]:[.5,.5,.5],d=new St,f=new gn(d,2),p=[(n[0]+r[0])/2,(n[1]+r[1])/2,(n[2]+r[2])/2];t===`box`?f.solid(`box`,{size:[r[0]-n[0],r[1]-n[1],r[2]-n[2]],at:p,color:u}):f.solid(`sphere`,{radius:Math.max(r[0]-n[0],r[1]-n[1],r[2]-n[2])/3,at:p,color:u,sides:4});let m=Qn(d);if(c*2>l)for(let e=5;e<m.length;e+=6)m[e]<256&&(m[e]=m[e]+256);return m}function Zn(e){let t=[1/0,1/0,1/0],n=[-1/0,-1/0,-1/0];for(let r=0;r+6<=e.length;r+=6)for(let i=0;i<3;i++)t[i]=Math.min(t[i],e[r+i]),n[i]=Math.max(n[i],e[r+i]);return[t,n]}function Qn(e){return Dt(t=>{t.adopt(e)})}const $n=/* @__PURE__ */ new Map,er=180/Math.PI,tr=e=>e?typeof e==`string`?[e]:e:[],nr=e=>new Set(Object.values(e.traits).map(e=>e.anchors).filter(e=>!!e));var rr=class{name;about;traits;constructor(e,t=!0){if(!e?.name||typeof e.name!=`string`)throw new W(`E-DATA-001`,`a kind needs a name`,{fix:`kind({ name: 'shed', traits: { door: { anchors: 'entry' } } })`});this.name=e.name,this.about=e.about??``,this.traits={...e.extends?.traits??{},...e.traits};for(let[t,n]of Object.entries(this.traits)){if(![n.anchors,n.parts,n.measure,n.param,n.data].filter(e=>e!==void 0).length)throw new W(`E-DATA-001`,`kind '${e.name}': trait '${t}' says nothing it is (anchors, parts, measure, param or data)`,{where:e.name,fix:`${t}: { anchors: 'entry' }`});if(n.per&&!this.traits[n.per])throw new W(`E-DATA-001`,`kind '${e.name}': trait '${t}' is per '${n.per}', which it does not have`,{where:e.name,fix:`traits: { ${n.per}: { … } }`})}t&&$n.set(this.name,this)}check(e){let t=[],n=cr(e),r=/* @__PURE__ */ new Map,i=(e,n,r)=>t.push(`${this.name} › ${e}${n.about?` (${n.about})`:``}: ${r}`);for(let[t,a]of Object.entries(this.traits))if(a.data===void 0||a.anchors!==void 0||a.parts!==void 0||a.measure!==void 0||a.param!==void 0){if(a.anchors!==void 0){let o=e.anchors.filter(e=>e.kind===a.anchors);r.set(t,o.length),ir(o.length,a)||(!a.optional||o.length)&&i(t,a,ar(o.length,a,`'${a.anchors}' anchor`));for(let e of o)for(let r of tr(a.where)){let o=or(e,r,n);o&&i(t,a,`anchor '${e.name}' ${o}`)}}if(a.parts!==void 0){let n=e.parts.filter(e=>e.name.startsWith(a.parts));r.set(t,n.length),!ir(n.length,a)&&(!a.optional||n.length)&&i(t,a,ar(n.length,a,`part named '${a.parts}…'`))}if(a.measure===`roof`&&!a.optional){let n=lr(e);(!n||n.surface<=0)&&i(t,a,`no roof: nothing faces up at its top (a group named 'roof', or the faces up there)`)}a.measure===`footprint`&&(n.width<=0||n.depth<=0)&&i(t,a,`nothing stands up from the ground`),a.param!==void 0&&!a.optional&&e.params&&!(a.param in e.params)&&!a.guess&&i(t,a,`no param '${a.param}'`)}for(let[e,t]of Object.entries(this.traits))t.per&&r.has(e)&&r.has(t.per)&&r.get(e)!==r.get(t.per)&&i(e,t,`${r.get(e)}, one per ${t.per} wanted (${r.get(t.per)})`);return{ok:!t.length,problems:t}}checkData(e){let t=[];for(let[n,r]of Object.entries(this.traits))r.data!==void 0&&r.def===void 0&&e[r.data]===void 0&&t.push(`${this.name} › ${n}: no '${r.data}' in its data`);return{ok:!t.length,problems:t}}};const ir=(e,t)=>e>=(t.min??1)&&e<=(t.max??1/0),ar=(e,t,n)=>`${e} ${n}${e===1?``:`s`}, ${t.max!==void 0&&e>t.max?`at most ${t.max}`:`at least ${t.min??1}`} wanted`;function or(e,t,n){let[r,i,a]=e.at,o=.05,s=r>n.min[0]+o&&r<n.max[0]-o&&a>n.min[1]+o&&a<n.max[1]-o;if(t===`ground`)return Math.abs(i)>.35?`is ${i.toFixed(2)} m up, not on the ground`:null;if(t===`up`)return i<.3?`is on the ground, not up`:null;let c=`the footprint (${n.from===`walls`?`the box of its 'walls' group`:`the box of all that stands over 0.3 m; a group named 'walls' sets it instead`}: x ${n.min[0].toFixed(1)} to ${n.max[0].toFixed(1)}, z ${n.min[1].toFixed(1)} to ${n.max[1].toFixed(1)})`;if(t===`outside`)return s?`is inside ${c}`:null;if(t===`inside`)return s?null:`is outside ${c}`;if(t===`front`)return a<(n.min[1]+n.max[1])/2?`is at the back, not the front (+z)`:e.heading!==void 0&&Math.abs(((e.heading-180)%360+540)%360-180)>60?`faces ${e.heading}°, not the shape (180°)`:null;let l=Math.max(n.min[0]-r,0,r-n.max[0]),u=Math.max(n.min[1]-a,0,a-n.max[1]),d=Math.hypot(l,u),f=s?Math.min(r-n.min[0],n.max[0]-r,a-n.min[1],n.max[1]-a):0;return d>.6||f>.6?`is ${Math.max(d,f).toFixed(2)} m from a wall`:null}const sr=e=>e.levels[0]instanceof Float32Array?e.levels[0]:null;function cr(e){let t=sr(e),n=(e.groups??[]).filter(e=>e.name===`walls`&&e.count>0),r=1/0,i=1/0,a=-1/0,o=-1/0,s=n.length?`walls`:`standing`,c=e=>{let n=t[e],s=t[e+2];n<r&&(r=n),n>a&&(a=n),s<i&&(i=s),s>o&&(o=s)};if(t&&n.length)for(let e of n)for(let t=e.first;t<e.first+e.count;t++)c(t*6);else if(t)for(let e=0;e<t.length;e+=6)t[e+1]>.3&&c(e);return Number.isFinite(r)||(r=e.bounds[0][0],i=e.bounds[0][2],a=e.bounds[1][0],o=e.bounds[1][2],s=`bounds`),{width:a-r,depth:o-i,min:[r,i],max:[a,o],from:s}}function lr(e){let t=sr(e);if(!t)return null;let n=(e.groups??[]).filter(e=>e.name===`roof`),r=e.bounds[1][1],i=e.bounds[0][1]+(r-e.bounds[0][1])*.5,a=n.length?n.map(e=>[e.first,e.first+e.count]):[[0,t.length/6]],o=0,s=0;for(let[e,r]of a)for(let a=e;a+2<r;a+=3){let e=e=>[t[(a+e)*6],t[(a+e)*6+1],t[(a+e)*6+2]],r=e(0),c=e(1),l=e(2),u=[c[0]-r[0],c[1]-r[1],c[2]-r[2]],d=[l[0]-r[0],l[1]-r[1],l[2]-r[2]],f=[u[1]*d[2]-u[2]*d[1],u[2]*d[0]-u[0]*d[2],u[0]*d[1]-u[1]*d[0]],p=Math.hypot(f[0],f[1],f[2]);if(p<1e-12)continue;let m=f[1]/p,h=p/2;m<.2||!n.length&&(r[1]+c[1]+l[1])/3<i||(o+=h,s+=h*Math.acos(Math.min(1,m))*er)}return o>0?{surface:o,tilt:s/o,top:r,...e.params?.roof===void 0?{}:{kind:String(e.params.roof)}}:null}function ur(e,t,n={}){let r=dr(t),i={},a=cr(e);for(let[t,o]of Object.entries(r.traits))o.anchors===void 0?o.parts===void 0?o.measure===`footprint`?i[t]=a:o.measure===`front`?i[t]={heading:0,at:[(a.min[0]+a.max[0])/2,0,a.max[1]]}:o.measure===`roof`?i[t]=lr(e):o.param===void 0?o.data!==void 0&&(i[t]=n[o.data]??o.def):i[t]=e.params?.[o.param]??(o.data===void 0?void 0:n[o.data])??o.guess?.(e)??o.def:i[t]=e.parts.filter(e=>e.name.startsWith(o.parts)):i[t]=e.anchors.filter(e=>e.kind===o.anchors);return i}function dr(e){if(e instanceof rr)return e;let t=Sr[e]??$n.get(e);if(!t)throw new W(`E-DATA-001`,`no kind '${e}' (the kinds: ${[...Object.keys(Sr),...$n.keys()].join(`, `)})`,{fix:`kind({ name: '${e}', traits: { … } }) first, or one of kinds.house, kinds.office, kinds.charger, kinds.vehicle`});return t}const fr=e=>e instanceof rr,pr=e=>Array.isArray(e)?e.map(e=>({...e,at:[...e.at]})):Object.entries(e).map(([e,t])=>({name:e,...t,at:[...t.at]}));function mr(e,t,n={}){let r={...e,kind:dr(t).name,...n.params?{params:{...e.params??{},...n.params}}:{}};return n.anchors&&(r.anchors=pr(typeof n.anchors==`function`?n.anchors(r):n.anchors)),r}const hr=e=>new rr(e),gr=e=>new rr(e,!1),_r=e=>Math.max(1,Math.round((e.bounds[1][1]-e.bounds[0][1])*.8/3)),vr=gr({name:`house`,about:`a home: one household, a few cars`,traits:{footprint:{measure:`footprint`,about:`the box of its walls`},front:{measure:`front`,about:`the side to its street: +z`},levels:{param:`levels`,guess:_r,about:`its floors`},floor:{param:`floor`,guess:()=>3,about:`a floor's height, m`},roof:{measure:`roof`,about:`its kind, surface and tilt`},entries:{anchors:`entry`,where:[`ground`,`front`],about:`where one walks in, from its front`},parking:{anchors:`parking`,where:[`ground`,`outside`],about:`where a car stands`},service:{anchors:`service`,where:`ground`,about:`a charger or a meter: on a wall or a post`},occupants:{data:`occupants`,def:3,about:`who lives there`}}}),yr=gr({name:`office`,about:`a workplace: many doors, a car park`,traits:{footprint:{measure:`footprint`,about:`the box of its walls`},front:{measure:`front`,about:`the side to its street: +z`},levels:{param:`levels`,guess:_r,about:`its floors`},floor:{param:`floor`,guess:()=>3.4,about:`a floor's height, m`},roof:{measure:`roof`,about:`its kind, surface and tilt`},entries:{anchors:`entry`,min:2,where:[`ground`,`front`],about:`its doors, at its front`},parking:{anchors:`parking`,min:4,where:[`ground`,`outside`],about:`its car park's bays`},service:{anchors:`service`,where:[`ground`],about:`chargers at its bays`},workers:{data:`workers`,def:40,about:`who works there`}}}),br=gr({name:`charger`,about:`a place to charge electric vehicles: a station or a wall charger`,traits:{bays:{anchors:`parking`,where:`ground`,about:`where a car stands to charge`},ports:{param:`ports`,about:`its ports`},power:{param:`power`,about:`kW`},connector:{param:`connector`,about:`its plug`},lights:{anchors:`light`,per:`bays`,where:`up`,about:`each bay's status light`}}}),xr=gr({name:`vehicle`,about:`a road vehicle`,traits:{wheels:{parts:`wheel`,min:2,about:`its wheels (they spin, the front ones steer)`},doors:{parts:`door`,about:`its doors (they open)`},seats:{anchors:`seat`,about:`where one sits`},entries:{anchors:`entry`,where:[`ground`,`outside`],about:`where one gets in`}}}),Sr={house:vr,office:yr,charger:br,vehicle:xr},Cr={house:vr,office:yr,charger:br,vehicle:xr,of:dr},wr=(e,t,n=(e+t)/2)=>({kind:`num`,min:e,max:t,def:n}),Tr=(e,t,n=Math.round((e+t)/2))=>({kind:`int`,min:e,max:t,def:n}),Er=(e,t=e[0])=>({kind:`choice`,options:e,def:t}),Dr=e=>({kind:`color`,def:e}),Or=(e=!1)=>({kind:`flag`,def:e}),kr=(e=``,t=24)=>({kind:`text`,def:e,max:t});var Ar=class{id;spec;cache=/* @__PURE__ */ new Map;cacheBytes=0;version=0;watchers=/* @__PURE__ */ new Set;watch(e){return this.watchers.add(e),()=>this.watchers.delete(e)}update(e){this.spec={...this.spec,...e},this.cache.clear(),this.cacheBytes=0,this.levels.clear(),this.version++;for(let e of[...this.watchers])try{e()}catch(e){console.error(e)}return this}constructor(e){if(typeof e?.build!=`function`)throw new W(`E-DATA-001`,`a shape needs build(b, p)`,{fix:`shape({ params: { h: num(1, 3, 2) }, build: (b, p) => b.solid('box', { size: [1, p.h, 1], at: [0, p.h / 2, 0] }) })`});this.spec=e,this.id=e.id??`shape-${++jr}`}params(e={}){let t={},n=this.spec.defaults?.(e)??{};for(let[r,i]of Object.entries(this.spec.params??{})){let a=e[r];t[r]=Lr(i,a===void 0?n[r]:a)}for(let[n,r]of Object.entries(e))n in t||(t[n]=r);return t}levels=/* @__PURE__ */ new Map;gone=new FinalizationRegistry(e=>{let t=this.levels.get(e);if(!t)return;let n=t.filter(e=>e.deref());n.length?this.levels.set(e,n):this.levels.delete(e)});sameAs(e){let t=Mr(e),n=this.levels.get(t);if(n)for(let t of n){let n=t.deref();if(n&&n!==e&&n.length===e.length&&Nr(n,e))return n}return n?n.push(new WeakRef(e)):this.levels.set(t,[new WeakRef(e)]),this.gone.register(e,t),e}make(e={},t=1){let n=this.params(e),r=`${t}|${JSON.stringify(n)}`,i=this.cache.get(r);if(i)return this.cache.delete(r),this.cache.set(r,i),i;let a=(e,r=this.spec.build)=>{let i=new St,a=new gn(i,e),o=Rr(t);return r(a,n,o),o=null,{shape:Qn(i),b:a}},o=a(0),s=this.spec.simple,c=s?a(1,typeof s==`function`?s:this.spec.build):null,l=[...new Set(o.b.parts.map(e=>e.name))].slice(0,15),u=[...new Set(o.b.groups.map(e=>e.name))].slice(0,7);for(let e of[o,c])e&&(e.shape=Fr(e.shape,e.b,l,u));let d=this.spec.far,f=typeof d==`function`?a(2,d):null;f&&(f.shape=Fr(f.shape,f.b,l,u));let p=this.spec.far,m=!p||p===`none`?null:f?f.shape:p===`box`||p===`dot`?Xn(o.shape,p):Gn(o.shape,{views:8}),h=c?c.shape:o.shape,g=this.spec.kind;i={levels:[o.shape,h,m??h].map(e=>e instanceof Float32Array?this.sameAs(e):e),parts:o.b.parts,anchors:o.b.anchors,triangles:[o.b.triangles,c?c.b.triangles:o.b.triangles,m instanceof Float32Array?m.length/18:m?2:c?c.b.triangles:o.b.triangles],bounds:Zn(o.shape),groups:o.b.groups,pieces:{parts:l.map(e=>{let t=o.b.parts.find(t=>t.name===e);return{name:e,pivot:t.pivot,...t.joint?{joint:t.joint}:{}}}),groups:u,emitters:o.b.emitters},params:n,...g?{kind:typeof g==`string`?g:g.name}:{},...this.spec.size?{size:this.spec.size}:{}},this.cache.set(r,i),this.cacheBytes+=Pr(i);for(let[e,t]of this.cache){if(this.cache.size<=256&&this.cacheBytes<=50331648)break;t!==i&&(this.cache.delete(e),this.cacheBytes-=Pr(t))}return i}into(e,t,n={}){let r=e.idNow;n.id!==void 0&&e.as(n.id),n.anchor&&e.on(n.anchor);let i=new gn(e,n.level??0);return i.tintColor=n.tint??null,i.frame({at:n.at??[0,0,0],turn:n.turn??0},()=>this.spec.build(i,this.params(t),Rr(n.seed??1))),e.as(r),e.on(null),i}check(e={}){let t=e.kind,n=fr(t)||typeof t==`string`&&!(this.spec.params&&`kind`in this.spec.params),r=n?t:this.spec.kind;if(n){let{kind:t,...n}=e;e=n}let i=[],a=[{}],o=/* @__PURE__ */ new Set,s=r?nr(dr(r)):/* @__PURE__ */ new Set;for(let[e,t]of Object.entries(this.spec.params??{}))if(t.kind===`num`||t.kind===`int`)a.push({[e]:t.min},{[e]:t.max});else if(t.kind===`choice`)for(let n of t.options)a.push({[e]:n});else t.kind===`flag`?a.push({[e]:!t.def}):t.kind===`text`&&a.push({[e]:``},{[e]:`W`.repeat(t.max)});let c={whole:0,simple:0,far:0},l=0;for(let t of a){let n;try{this.cache.clear(),this.cacheBytes=0,n=this.make({...e,...t}),l++}catch(e){i.push(`${JSON.stringify(t)}: ${e.message}`);continue}n.levels[0].length||i.push(`${JSON.stringify(t)}: nothing built`),n.levels.forEach((e,r)=>{if(!(!(e instanceof Float32Array)||r>0&&n.levels.indexOf(e)<r)){for(let n=0;n<e.length;n++)if(!Number.isFinite(e[n])){i.push(`${JSON.stringify(t)}: a number that is not one (NaN or infinite) in its ${[`whole`,`simple`,`far`][r]} level's vertices`);break}}}),n.triangles[1]>n.triangles[0]&&i.push(`${JSON.stringify(t)}: the simple level has more triangles (${n.triangles[1]}) than the whole (${n.triangles[0]})`);let[a,u]=n.bounds,d=Math.max(.5,Math.max(u[0]-a[0],u[1]-a[1],u[2]-a[2])*.25),f=e=>e.some((e,t)=>e<a[t]-d||e>u[t]+d);for(let e of n.anchors)!(e.kind&&s.has(e.kind))&&f(e.at)&&i.push(`${JSON.stringify(t)}: anchor '${e.name}' at ${e.at.map(e=>e.toFixed(2))} is outside the shape`);for(let e of n.parts)f(e.pivot)&&i.push(`${JSON.stringify(t)}: part '${e.name}' turns round a pivot outside the shape`),e.count||i.push(`${JSON.stringify(t)}: part '${e.name}' has nothing in it`);if(r)for(let e of dr(r).check(n).problems)o.has(e)||(o.add(e),i.push(`${JSON.stringify(t)}: ${e}`));c={whole:Math.max(c.whole,n.triangles[0]),simple:Math.max(c.simple,n.triangles[1]),far:Math.max(c.far,n.triangles[2])}}let u=this.spec.budget??{};return u.whole!==void 0&&c.whole>u.whole&&i.push(`whole: ${c.whole} triangles over its budget ${u.whole}`),u.simple!==void 0&&c.simple>u.simple&&i.push(`simple: ${c.simple} triangles over its budget ${u.simple}`),u.far!==void 0&&c.far>u.far&&i.push(`far: ${c.far} triangles over its budget ${u.far}`),this.cache.clear(),this.cacheBytes=0,{ok:!i.length,problems:i,triangles:c,builds:l}}};let jr=0;function Mr(e){let t=new Uint32Array(e.buffer,e.byteOffset,e.length),n=2166136261;for(let e=0;e<t.length;e++)n=Math.imul(n^t[e],16777619);return n>>>0}function Nr(e,t){let n=new Uint32Array(e.buffer,e.byteOffset,e.length),r=new Uint32Array(t.buffer,t.byteOffset,t.length);for(let e=0;e<n.length;e++)if(n[e]!==r[e])return!1;return!0}const Pr=e=>{let t=0,n=/* @__PURE__ */ new Set;for(let r of e.levels)r instanceof Float32Array&&!n.has(r)&&(n.add(r),t+=r.byteLength);return t};function Fr(e,t,n,r){if(!n.length&&!r.length)return e;let i=e.length/6,a=new Uint8Array(i);for(let e of[...t.parts].sort((e,t)=>t.count-e.count)){let t=n.indexOf(e.name)+1;if(t>0)for(let n=e.first;n<Math.min(i,e.first+e.count);n++)a[n]=a[n]&240|t}for(let e of t.groups){let t=r.indexOf(e.name)+1;if(t>0)for(let n=e.first;n<Math.min(i,e.first+e.count);n++)a[n]=a[n]&15|t<<4}for(let t=0;t<i;t++){let n=t*6+5;a[t]&&e[n]<65536&&(e[n]=e[n]%512+512*a[t])}return e}const Ir=e=>new Ar(e);function Lr(e,t){return t==null?e.def:e.kind===`num`?Math.max(e.min,Math.min(e.max,Number(t))):e.kind===`int`?Math.round(Math.max(e.min,Math.min(e.max,Number(t)))):e.kind===`choice`?e.options.includes(String(t))?String(t):e.def:e.kind===`flag`?!!t:e.kind===`text`?String(t).slice(0,e.max):String(t)}function Rr(e){let t=e*2654435761>>>0||1;return()=>(t^=t<<13,t^=t>>>17,t^=t<<5,t>>>=0,t/4294967296)}const zr=(e,t)=>{if(`kind`in t)return t.kind===`num`?{key:e,kind:`slider`,min:t.min,max:t.max,step:(t.max-t.min)/100,value:t.def}:t.kind===`int`?{key:e,kind:`stepper`,min:t.min,max:t.max,step:1,value:t.def}:t.kind===`choice`?{key:e,kind:`dropdown`,options:t.options,value:t.def}:t.kind===`color`?{key:e,kind:`colour`,value:t.def}:t.kind===`text`?{key:e,kind:`text`,size:t.max,max:t.max,value:t.def}:{key:e,kind:`switch`,value:t.def};if(!t||typeof t!=`object`||!(`type`in t))throw new W(`E-DATA-001`,`param '${e}' is neither a shape's (num, int, choice, color, flag, text) nor a world's ({ type, default })`,{where:e,fix:`params: { size: num(.3, 2, .6) } or { size: { type: 'number', range: [.3, 2], default: .6 } }`});let n=typeof t.get==`function`?t.get():t.default,r={...t.unit?{unit:t.unit}:{},...t.describe?{describe:t.describe}:{}};return t.type===`number`?t.range?{key:e,kind:`slider`,min:t.range[0],max:t.range[1],step:(t.range[1]-t.range[0])/100,value:n,...r}:{key:e,kind:`number`,value:n,...r}:t.type===`enum`?{key:e,kind:`dropdown`,options:t.options??[],value:n,...r}:t.type===`bool`?{key:e,kind:`switch`,value:!!n,...r}:t.type===`color`?{key:e,kind:`colour`,value:n,...r}:t.type===`vector`?{key:e,kind:`vector`,size:Array.isArray(t.default)?t.default.length:3,value:n,...t.range?{min:t.range[0],max:t.range[1]}:{},...r}:{key:e,kind:`text`,value:n,...r}},Br=e=>{let t=e=>e instanceof Ar||!!e&&typeof e.make==`function`&&typeof e.spec==`object`;if(t(e))return e.spec.params??{};let n=e;return t(n.shape)?n.shape.spec.params??{}:typeof n.params==`function`?n.params():e};function Vr(e){let t=Object.entries(Br(e)).map(([e,t])=>zr(e,t));return{controls:t,render(e,n){let r=document.createElement(`div`),i=Object.fromEntries(t.map(e=>[e.key,e.value]));for(let e of t){let t=document.createElement(`label`);if(t.style.display=`block`,t.textContent=`${e.key}${e.unit?` (${e.unit})`:``} `,e.kind===`vector`){let a=Array.isArray(e.value)?[...e.value]:Array.from({length:e.size??3},()=>0);a.forEach((r,o)=>{let s=document.createElement(`input`);s.type=`number`,s.value=String(r),s.style.width=`5em`,s.addEventListener(`input`,()=>{a[o]=Number(s.value),i[e.key]=[...a],n({...i})}),t.appendChild(s)}),r.appendChild(t);continue}let a;if(e.kind===`dropdown`){let t=document.createElement(`select`);for(let n of e.options??[]){let e=document.createElement(`option`);e.value=n,e.textContent=n,t.appendChild(e)}t.value=String(e.value),a=t}else{let t=document.createElement(`input`);t.type=e.kind===`colour`?`color`:e.kind===`switch`?`checkbox`:e.kind===`text`?`text`:e.kind===`number`?`number`:`range`,e.max!==void 0&&e.kind===`text`&&(t.maxLength=e.max),e.min!==void 0&&e.kind!==`text`&&(t.min=String(e.min)),e.max!==void 0&&e.kind!==`text`&&(t.max=String(e.max)),e.step!==void 0&&(t.step=String(e.step)),e.kind===`switch`?t.checked=!!e.value:t.value=String(e.value),a=t}a.addEventListener(`input`,()=>{i[e.key]=e.kind===`switch`?a.checked:e.kind===`slider`||e.kind===`stepper`||e.kind===`number`?Number(a.value):a.value,n({...i})}),t.appendChild(a),r.appendChild(t)}return e.appendChild(r),r}}}let Hr=0;var Ur=class{kind=`source`;id;attribution;version=0;shapeVersion=0;failure=null;list=[];byId=/* @__PURE__ */ new Map;watchers=/* @__PURE__ */ new Set;recorders=/* @__PURE__ */ new Set;started=null;busy=0;constructor(e={}){this.id=e.id??`${this.constructor.name.toLowerCase()}-${++Hr}`,this.attribution=e.attribution??``}features(){return this.list}get byView(){return!1}get(e){return this.byId.get(e)??this.byId.get(String(e))??(typeof e==`string`&&e!==``&&!Number.isNaN(Number(e))?this.byId.get(Number(e)):void 0)}get size(){return this.list.length}start(e){return this.started??=this.track(this.load(e)).catch(t=>{this.failure=t instanceof Error?t:Error(String(t)),e.error(t)}),this.started}unstart(){this.started=null}get ready(){return this.started??Promise.resolve()}frame(e,t){}status(){return{pending:this.busy?[`source '${this.id}' loading`]:[],failed:this.failure?[{what:`source '${this.id}'`,error:this.failure.message}]:[]}}track(e){return this.busy++,e.finally(()=>{this.busy--})}watch(e,t={}){let n=t.now?this.recorders:this.watchers;return n.add(e),()=>n.delete(e)}get watching(){return this.watchers.size+this.recorders.size}inputs(){return[]}reopen(){}close(){}replace(e,t=!0){this.list=e,this.byId=new Map(e.map(e=>[e.id,e])),this.changed(t)}changed(e){this.version++,e&&this.shapeVersion++;for(let e of[...this.recorders])try{e()}catch(e){console.error(e)}Ue(this,()=>{for(let e of[...this.watchers])try{e()}catch(e){console.error(e)}})}join(e,t){return new Wr(this,e,t)}},Wr=class extends Ur{keyOf;rowKeyOf;seen=[-1,-1,-1];shapes;rows;constructor(e,t,n){if(super({id:`${e.id}+${t.id}`,attribution:[e.attribution,t.attribution].filter(Boolean).join(` · `)}),this.shapes=e,this.rows=t,!n||typeof n.on!=`string`&&typeof n.on!=`function`)throw new W(`E-DATA-001`,`join needs { on }`,{where:this.id,fix:`shapes.join(rows, { on: 'suburb_code' })`});let r=e=>typeof e==`function`?e:t=>e===`id`?t.id:t.props[e];this.keyOf=r(n.on),this.rowKeyOf=r(n.rowsOn??n.on),this.reopen()}offs=[];inputs(){return[this.shapes,this.rows]}reopen(){this.offs.length||(this.offs=[this.shapes.watch(()=>this.sync(),{now:!0}),this.rows.watch(()=>this.sync(),{now:!0})]),this.sync()}close(){for(let e of this.offs)e();this.offs=[]}features(){return this.offs.length||this.sync(),super.features()}get(e){return this.offs.length||this.sync(),super.get(e)}get size(){return this.offs.length||this.sync(),super.size}async load(e){await Promise.all([this.shapes.start(e),this.rows.start(e)]),this.sync()}frame(e,t){this.shapes.frame(e,t),this.rows.frame(e,t)}status(){let e=this.shapes.status(),t=this.rows.status();return{pending:[...e.pending,...t.pending],failed:[...e.failed,...t.failed]}}get ready(){return Promise.all([this.shapes.ready,this.rows.ready]).then(()=>{})}get byView(){return this.shapes.byView||this.rows.byView}matched=0;sync(){if(this.seen[0]===this.shapes.version&&this.seen[1]===this.rows.version)return;let e=this.seen[2]!==this.shapes.shapeVersion;this.seen=[this.shapes.version,this.rows.version,this.shapes.shapeVersion];let t=/* @__PURE__ */ new Map;for(let e of this.rows.features()){let n=this.rowKeyOf(e);n!=null&&t.set(String(n),e)}let n=0,r=this.shapes.features().map(e=>{let r=t.get(String(this.keyOf(e)));return r&&n++,{id:e.id,geometry:e.geometry,cuts:e.cuts,props:r?{...e.props,...r.props}:{...e.props}}});this.matched=n,this.replace(r,e)}};const Gr=/* @__PURE__ */ new Map;function Kr(e,t=`#ffffff`){if(e==null)return Kr(t);if(Array.isArray(e))return[e[0]??1,e[1]??1,e[2]??1,e[3]??1];let n=e,r=Gr.get(n);if(r)return r;let i=null,a=/^#([0-9a-f]{3,4})$/i.exec(n);if(a){let e=a[1];i=[parseInt(e[0]+e[0],16)/255,parseInt(e[1]+e[1],16)/255,parseInt(e[2]+e[2],16)/255,e[3]?parseInt(e[3]+e[3],16)/255:1]}if(a=i?null:/^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(n),a){let e=parseInt(a[1],16);i=[(e>>16&255)/255,(e>>8&255)/255,(e&255)/255,a[2]?parseInt(a[2],16)/255:1]}if(a=i?null:/^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:[\s,/]+([\d.]+%?))?\s*\)$/i.exec(n),a){let e=a[4]===void 0?1:a[4].endsWith(`%`)?parseFloat(a[4])/100:+a[4];i=[a[1]/255,a[2]/255,a[3]/255,e]}if(!i&&n===`transparent`&&(i=[0,0,0,0]),!i)throw new W(`E-DATA-001`,`colour '${n}' is not one aslnut reads`,{fix:`write '#ff8800', '#ff880080', 'rgb(255 136 0)' or [1, 0.53, 0]`});return Gr.size>4096&&Gr.clear(),Gr.set(n,i),i}const qr=e=>`#${e.slice(0,3).map(e=>Math.round(Math.max(0,Math.min(1,e))*255).toString(16).padStart(2,`0`)).join(``)}${e[3]<1?Math.round(Math.max(0,e[3])*255).toString(16).padStart(2,`0`):``}`,Jr=(e,t,n)=>[e[0]+(t[0]-e[0])*n,e[1]+(t[1]-e[1])*n,e[2]+(t[2]-e[2])*n,e[3]+(t[3]-e[3])*n],Yr={viridis:[`#440154`,`#482878`,`#3e4989`,`#31688e`,`#26828e`,`#1f9e89`,`#35b779`,`#6ece58`,`#b5de2b`,`#fde725`],inferno:[`#000004`,`#1b0c41`,`#4a0c6b`,`#781c6d`,`#a52c60`,`#cf4446`,`#ed6925`,`#fb9b06`,`#f7d13d`,`#fcffa4`],magma:[`#000004`,`#180f3d`,`#440f76`,`#721f81`,`#9e2f7f`,`#cd4071`,`#f1605d`,`#fd9668`,`#feca8d`,`#fcfdbf`],plasma:[`#0d0887`,`#46039f`,`#7201a8`,`#9c179e`,`#bd3786`,`#d8576b`,`#ed7953`,`#fb9f3a`,`#fdca26`,`#f0f921`],turbo:[`#30123b`,`#4662d7`,`#36aaf9`,`#1ae4b6`,`#72fe5e`,`#c8ef34`,`#faba39`,`#f66b19`,`#ca2a04`,`#7a0403`],cividis:[`#00224e`,`#123570`,`#3b496c`,`#575d6d`,`#707173`,`#8a8779`,`#a69d75`,`#c4b56c`,`#e4cf5b`,`#fee838`],reds:[`#fff5f0`,`#fee0d2`,`#fcbba1`,`#fc9272`,`#fb6a4a`,`#ef3b2c`,`#cb181d`,`#a50f15`,`#67000d`],blues:[`#f7fbff`,`#deebf7`,`#c6dbef`,`#9ecae1`,`#6baed6`,`#4292c6`,`#2171b5`,`#08519c`,`#08306b`],greens:[`#f7fcf5`,`#e5f5e0`,`#c7e9c0`,`#a1d99b`,`#74c476`,`#41ab5d`,`#238b45`,`#006d2c`,`#00441b`],oranges:[`#fff5eb`,`#fee6ce`,`#fdd0a2`,`#fdae6b`,`#fd8d3c`,`#f16913`,`#d94801`,`#a63603`,`#7f2704`],purples:[`#fcfbfd`,`#efedf5`,`#dadaeb`,`#bcbddc`,`#9e9ac8`,`#807dba`,`#6a51a3`,`#54278f`,`#3f007d`],greys:[`#ffffff`,`#f0f0f0`,`#d9d9d9`,`#bdbdbd`,`#969696`,`#737373`,`#525252`,`#252525`,`#000000`],ylorrd:[`#ffffcc`,`#ffeda0`,`#fed976`,`#feb24c`,`#fd8d3c`,`#fc4e2a`,`#e31a1c`,`#bd0026`,`#800026`],ylgnbu:[`#ffffd9`,`#edf8b1`,`#c7e9b4`,`#7fcdbb`,`#41b6c4`,`#1d91c0`,`#225ea8`,`#253494`,`#081d58`],rdylgn:[`#a50026`,`#d73027`,`#f46d43`,`#fdae61`,`#fee08b`,`#d9ef8b`,`#a6d96a`,`#66bd63`,`#1a9850`,`#006837`],rdbu:[`#67001f`,`#b2182b`,`#d6604d`,`#f4a582`,`#fddbc7`,`#d1e5f0`,`#92c5de`,`#4393c3`,`#2166ac`,`#053061`],spectral:[`#9e0142`,`#d53e4f`,`#f46d43`,`#fdae61`,`#fee08b`,`#e6f598`,`#abdda4`,`#66c2a5`,`#3288bd`,`#5e4fa2`],category:[`#4e79a7`,`#f28e2b`,`#e15759`,`#76b7b2`,`#59a14f`,`#edc948`,`#b07aa1`,`#ff9da7`,`#9c755f`,`#bab0ac`],set2:[`#66c2a5`,`#fc8d62`,`#8da0cb`,`#e78ac3`,`#a6d854`,`#ffd92f`,`#e5c494`,`#b3b3b3`],pastel:[`#fbb4ae`,`#b3cde3`,`#ccebc5`,`#decbe4`,`#fed9a6`,`#ffffcc`,`#e5d8bd`,`#fddaec`]},Xr=/* @__PURE__ */ new Set([`category`,`set2`,`pastel`]);function Zr(e,t={}){let n,r,i=`custom`;if(typeof e==`function`)n=t=>qr(Kr(e(t))),r=Array.from({length:9},(e,t)=>n(t/8));else{let t;if(typeof e==`string`){let n=e.toLowerCase(),r=Yr[n];if(!r)throw new W(`E-DATA-001`,`no ramp '${e}'`,{fix:`use one of ${Object.keys(Yr).join(`, `)}, or give colours`});i=n,t=r.map((e,t)=>[t/(r.length-1),e])}else if(Array.isArray(e)&&e.length&&Array.isArray(e[0])&&typeof e[0][0]==`number`&&e[0].length===2)t=e.map(([e,t])=>[e,qr(Kr(t))]);else if(Array.isArray(e)&&e.length){let n=e;t=n.map((e,t)=>[n.length===1?0:t/(n.length-1),qr(Kr(e))])}else throw new W(`E-DATA-001`,`a ramp is a name, colours, [t, colour] stops or a function`,{fix:`ramp('viridis') · ramp(['#fee', '#c00'])`});t.sort((e,t)=>e[0]-t[0]);let a=t.map(([e,t])=>[e,Kr(t)]);r=t.map(e=>e[1]),n=e=>{if(!(e>=a[0][0]))return qr(a[0][1]);for(let t=1;t<a.length;t++)if(e<=a[t][0]){let n=a[t-1],r=a[t];return qr(Jr(n[1],r[1],(e-n[0])/(r[0]-n[0]||1)))}return qr(a[a.length-1][1])}}let a=t.reverse?e=>n(1-e):n;return Object.defineProperties(a,{stops:{value:t.reverse?[...r].reverse():r},name:{value:i}}),a}function Qr(e,t){if(typeof e==`string`&&Xr.has(e.toLowerCase())){let n=Yr[e.toLowerCase()];return Array.from({length:t},(e,t)=>n[t%n.length])}let n=Zr(e);return Array.from({length:t},(e,r)=>n(t===1?.5:r/(t-1)))}const $r=e=>typeof e==`number`?e:typeof e==`string`&&e.trim()!==``?Number(e):NaN,ei=e=>Array.isArray(e)&&e.length===2&&typeof e[0]==`number`&&typeof e[1]==`number`,ti=e=>Math.abs(e)>=1e3?Math.round(e).toLocaleString(`en`):Math.abs(e)>=10?e.toFixed(0):Math.abs(e)>=1?e.toFixed(1):e.toPrecision(2);function ni(e,t,n,r,i,a,o,s,c){if(typeof t!=`function`)throw new W(`E-DATA-001`,`scale.${e} needs a function of the feature`,{fix:`scale.${e}(d => d.props.rate, { … })`});let l=n.missing??(ei(n.range)?0:`#8a8f98`),u=((e,n)=>{let i=$r(t(e,n));return Number.isFinite(i)?r(i):l}),d;return Object.defineProperties(u,{prepare:{value:(e,n)=>{(n===void 0||n!==d)&&(d=n,i(e.map(e=>$r(t(e))).filter(Number.isFinite)))}},auto:{value:c},legend:{value:e=>({...a(e?.format??ti),missing:typeof l==`string`?l:void 0})},breaks:{get:o},kind:{value:e},version:{get:()=>s.version}}),u}function ri(e){let t=e.range??`viridis`;return ei(t)?(e=>t[0]+(t[1]-t[0])*e):Zr(t,{reverse:e.reverse})}function ii(e,t,n={}){let r={version:0},i=ri(n),a=Array.isArray(n.domain)&&n.domain.length===2?n.domain:[0,1],o=e===`log`?e=>Math.log10(Math.max(1e-12,e)):e===`sqrt`?e=>Math.sqrt(Math.max(0,e)):e=>e,s=e=>{let t=o(a[0]),r=o(a[1]),s=r===t?.5:(o(e)-t)/(r-t);return n.clamp!==!1&&(s=Math.max(0,Math.min(1,s))),i(s)};return ni(e,t,n,s,e=>{if(n.domain&&n.domain!==`auto`||!e.length)return;let t=1/0,i=-1/0;for(let n of e)n<t&&(t=n),n>i&&(i=n);(t!==a[0]||i!==a[1])&&(a=[t,i],r.version++)},t=>({kind:`continuous`,domain:a,stops:ei(n.range)?void 0:Zr(n.range??`viridis`,{reverse:n.reverse}).stops,items:[0,.5,1].map(n=>{let r=e===`log`?10**(o(a[0])+(o(a[1])-o(a[0]))*n):e===`sqrt`?(o(a[0])+(o(a[1])-o(a[0]))*n)**2:a[0]+(a[1]-a[0])*n,i=s(r);return{label:t(r),value:r,...typeof i==`string`?{color:i}:{size:i}}})}),()=>a,r,!(Array.isArray(n.domain)&&n.domain.length===2))}function ai(e,t){let n=e.length;if(n<=t)return e.slice(1);let r=Array.from({length:n+1},()=>Array(t+1).fill(0)),i=Array.from({length:n+1},()=>Array(t+1).fill(1/0));for(let e=1;e<=t;e++)r[1][e]=1,i[1][e]=0;for(let a=2;a<=n;a++){let n=0,o=0,s=0;for(let c=1;c<=a;c++){let l=a-c+1,u=e[l-1];o+=u*u,n+=u,s++;let d=o-n*n/s;if(l>1)for(let e=2;e<=t;e++)i[a][e]>=d+i[l-1][e-1]&&(r[a][e]=l,i[a][e]=d+i[l-1][e-1])}r[a][1]=1,i[a][1]=o-n*n/s}let a=[],o=n;for(let n=t;n>=2;n--){let t=r[o][n];a.unshift(e[t-1]),o=t-1}return a}function oi(e,t,n){let r=(t-e)/Math.max(1,n)||1,i=10**Math.floor(Math.log10(r)),a=[1,2,2.5,5,10].map(e=>e*i).find(e=>e>=r)??r,o=[];for(let r=Math.floor(e/a)*a+a;r<t-1e-9*a&&o.length<n-1;r+=a)o.push(+r.toPrecision(12));return o}function si(e,t,n={}){let r={version:0},i=Array.isArray(n.domain)&&e!==`quantize`&&(e===`threshold`||n.domain.length>=1),a=i?[...n.domain]:[];if(e===`threshold`&&!a.length)throw new W(`E-DATA-001`,`scale.threshold needs domain: the breaks`,{fix:`scale.threshold(d => d.props.v, { domain: [10, 50, 100], range: "reds" })`});let o=0,s=1,c=()=>i||(e===`pretty`||e===`jenks`)&&a.length?a.length+1:n.n??5,l=()=>{let e=n.range??`reds`;if(Array.isArray(e)&&!ei(e)&&e.length>=c()&&typeof e[0]!=`object`)return e.slice(0,c());if(ei(e))return Array.from({length:c()},(t,n)=>e[0]+(e[1]-e[0])*n/Math.max(1,c()-1));let t=Qr(e,c());return n.reverse?t.reverse():t},u=l();return ni(e,t,n,e=>{let t=0;for(;t<a.length&&e>=a[t];)t++;return u[Math.min(t,u.length-1)]},t=>{if(i||!t.length){i&&t.length&&(o=Math.min(...t),s=Math.max(...t));return}let d=[...t].sort((e,t)=>e-t),f=e===`pretty`||e===`jenks`?n.n??5:c();o=d[0],s=d[d.length-1];let p=e===`quantile`?Array.from({length:f-1},(e,t)=>d[Math.min(d.length-1,Math.floor((t+1)*d.length/f))]):e===`jenks`?ai(d,f):e===`pretty`?oi(o,s,f):Array.from({length:f-1},(e,t)=>Array.isArray(n.domain)?n.domain[0]+(n.domain[1]-n.domain[0])*(t+1)/f:o+(s-o)*(t+1)/f);p.join()!==a.join()&&(a=p,u=l(),r.version++)},e=>({kind:`classes`,items:u.map((t,n)=>{if(!a.length)return{label:`…`,...typeof t==`string`?{color:t}:{size:t}};let r=n===0?o:a[n-1]??o,i=n<a.length?a[n]:s;return{from:r,to:i,label:n===0?`< ${e(a[0]??s)}`:n===u.length-1?`≥ ${e(a[n-1]??o)}`:`${e(r)} – ${e(i)}`,...typeof t==`string`?{color:t}:{size:t}}})}),()=>a,r,!i&&!(e===`quantize`&&Array.isArray(n.domain)&&n.domain.length===2))}function ci(e,t={}){let n={version:0},r=Array.isArray(t.domain)?t.domain.map(String):t.map?Object.keys(t.map):[],i=[],a=()=>{let e=t.range;i=Array.isArray(e)&&!ei(e)&&e.length&&(e.length>=r.length||typeof e[0]!=`string`)?e:Qr(e??`category`,Math.max(1,r.length))};a();let o=t.missing??`#8a8f98`,s=t.other??`#8a8f98`,c=((n,a)=>{let c=e(n,a);if(c==null||c===``)return o;let l=String(c);if(t.map&&l in t.map)return t.map[l];let u=r.indexOf(l);return u>=0?i[u%i.length]:s}),l;return Object.defineProperties(c,{auto:{value:!Array.isArray(t.domain)&&!t.map},prepare:{value:(i,o)=>{if(o!==void 0&&o===l||(l=o,Array.isArray(t.domain)||t.map))return;let s=new Set(r),c=!1;for(let t of i){let n=e(t);if(n==null||n===``)continue;let i=String(n);s.has(i)||(s.add(i),r.push(i),c=!0)}c&&(a(),n.version++)}},legend:{value:()=>({kind:`categories`,items:r.map((e,n)=>({label:e,value:e,color:String(t.map?.[e]??i[n%i.length])})),missing:String(o)})},breaks:{get:()=>r},kind:{value:`category`},version:{get:()=>n.version}}),c}const li={linear:(e,t)=>ii(`linear`,e,t),log:(e,t)=>ii(`log`,e,t),sqrt:(e,t)=>ii(`sqrt`,e,t),quantile:(e,t)=>si(`quantile`,e,t),quantize:(e,t)=>si(`quantize`,e,t),threshold:(e,t)=>si(`threshold`,e,t),jenks:(e,t)=>si(`jenks`,e,t),pretty:(e,t)=>si(`pretty`,e,t),category:(e,t)=>ci(e,t)},ui=e=>typeof e==`function`&&typeof e.prepare==`function`&&typeof e.legend==`function`,di=(e,t=!1)=>({k:`scalar`,s:e,...t?{abstract:t}:{}}),fi=(e,t=`f32`)=>({k:`vec`,n:e,s:t}),pi=di(`f32`),mi=di(`bool`),hi={k:`void`};function gi(e){return new Ei(Si(e,0),``).type()}function J(e){switch(e.k){case`scalar`:return e.s;case`vec`:return`vec${e.n}${e.s===`bool`?`<bool>`:e.s[0]}`;case`mat`:return`mat${e.c}x${e.r}f`;case`array`:return`array<${J(e.of)}, ${e.n}>`;case`struct`:return e.name;case`void`:return`void`}}function Y(e){switch(e.k){case`scalar`:return{f32:`float`,i32:`int`,u32:`uint`,bool:`bool`}[e.s];case`vec`:return`${{f32:``,i32:`i`,u32:`u`,bool:`b`}[e.s]}vec${e.n}`;case`mat`:return e.c===e.r?`mat${e.c}`:`mat${e.c}x${e.r}`;case`array`:return`${Y(e.of)}[${e.n}]`;case`struct`:return e.name;case`void`:return`void`}}const _i=(e,t)=>e.k===t.k?e.k===`scalar`&&t.k===`scalar`?e.s===t.s:e.k===`vec`&&t.k===`vec`?e.n===t.n&&e.s===t.s:e.k===`mat`&&t.k===`mat`?e.c===t.c&&e.r===t.r:e.k===`array`&&t.k===`array`?e.n===t.n&&_i(e.of,t.of):e.k===`struct`&&t.k===`struct`?e.name===t.name:e.k===`void`:!1,X=e=>e.k===`scalar`||e.k===`vec`?e.s:e.k===`mat`?`f32`:null,vi=e=>e.k===`scalar`&&!!e.abstract,yi=e=>X(e)===`f32`,bi=(e,t)=>vi(e)?di(t):e,xi=/* @__PURE__ */ `-> <<= >>= += -= *= /= %= &= |= ^= ++ -- == != <= >= && || << >> ( ) { } [ ] , ; : . = < > + - * / % ! & | ^ ~ @`.split(` `);function Si(e,t){let n=[],r=0,i=t+1,a=1,o=t=>{for(let n=0;n<t;n++)e[r]===`
`?(i++,a=1):a++,r++};for(;r<e.length;){let t=e[r];if(/\s/.test(t)){o(1);continue}if(e.startsWith(`//`,r)){for(;r<e.length&&e[r]!==`
`;)o(1);continue}if(e.startsWith(`/*`,r)){let t=e.indexOf(`*/`,r+2);o((t<0?e.length:t+2)-r);continue}let s={line:i,col:a},c=/^(?:0[xX][0-9a-fA-F]+[iu]?|(?:\d+\.\d*|\.\d+|\d+)(?:[eE][+-]?\d+)?[fhiu]?)/.exec(e.slice(r));if(c&&/[\d.]/.test(t)&&(t!==`.`||/\d/.test(e[r+1]??``))){n.push({t:`num`,v:c[0],...s}),o(c[0].length);continue}let l=/^[A-Za-z_][A-Za-z0-9_]*/.exec(e.slice(r));if(l){n.push({t:`id`,v:l[0],...s}),o(l[0].length);continue}let u=xi.find(t=>e.startsWith(t,r));if(u)n.push({t:`op`,v:u,...s}),o(u.length);else throw new Ci(`unexpected character '${t}'`,s.line,s.col,`remove it; the shader language is a WGSL subset`)}return n.push({t:`eof`,v:``,line:i,col:a}),n}var Ci=class extends W{line;col;constructor(e,t,n,r,i=``){super(`E-SHADER-001`,`line ${t}, col ${n}: ${e}`,{fix:r,where:i}),this.name=`ShaderError`,this.line=t,this.col=n}};const wi={"||":1,"&&":2,"|":3,"^":4,"&":5,"==":6,"!=":6,"<":7,"<=":7,">":7,">=":7,"<<":8,">>":8,"+":9,"-":9,"*":10,"/":10,"%":10},Ti=/^(f32|i32|u32|bool|vec[234][fiu]?|mat[234]x[234]f?|array)$/;var Ei=class{p=0;toks;where;constructor(e,t){this.toks=e,this.where=t}get cur(){return this.toks[this.p]}err(e,t=this.cur,n=`check the WGSL syntax at that point`){throw new Ci(e,t.line,t.col,n,this.where)}is(e){return this.cur.v===e&&this.cur.t!==`num`}eat(e){return this.is(e)||this.err(`expected '${e}', found '${this.cur.v||`the end`}'`,this.cur,e===`;`?`end the statement with ;`:`add '${e}'`),this.toks[this.p++]}id(){return this.cur.t!==`id`&&this.err(`expected a name, found '${this.cur.v||`the end`}'`),this.toks[this.p++]}get done(){return this.cur.t===`eof`}type(){let e=this.id(),t=e.v;if(t===`f32`||t===`i32`||t===`u32`||t===`bool`)return di(t);let n=/^vec([234])([fiu])$/.exec(t);if(n)return fi(+n[1],{f:`f32`,i:`i32`,u:`u32`}[n[2]]);if(n=/^vec([234])$/.exec(t),n){this.eat(`<`);let t=this.type();return this.eat(`>`),t.k!==`scalar`&&this.err(`a vector holds scalars`,e),fi(+n[1],t.s)}if(n=/^mat([234])x([234])(f?)$/.exec(t),n){if(!n[3]){this.eat(`<`);let t=this.type();this.eat(`>`),(t.k!==`scalar`||t.s!==`f32`)&&this.err(`matrices are of f32`,e)}return{k:`mat`,c:+n[1],r:+n[2]}}if(t===`array`){this.eat(`<`);let e=this.type();this.eat(`,`);let t=this.cur;return t.t!==`num`&&this.err(`an array needs a fixed size`,t,`write array<f32, 4>`),this.p++,this.eat(`>`),{k:`array`,of:e,n:parseInt(t.v,t.v.startsWith(`0x`)?16:10)}}return(t===`f16`||/^vec[234]h$/.test(t))&&this.err(`'${t}' (16-bit floats) is not in the shader subset`,e,`use f32`),/^texture|^sampler/.test(t)&&this.err(`'${t}': textures are not in the shader subset yet`,e,`use the inputs and params the hook gives`),/^ptr$|^atomic$/.test(t)&&this.err(`'${t}' is not in the shader subset`,e,`use plain values`),this.err(`unknown type '${t}'`,e,`use f32, i32, u32, bool, vec2f..vec4f, vec2i.., vec2u.., mat2x2f..mat4x4f or array<T, N>`)}module(){let e=[],t=[];for(;!this.done;)if(this.is(`@`)&&this.err(`attributes (@…) are not in the shader subset`,this.cur,`write plain functions`),this.is(`fn`)){let t=this.eat(`fn`),n=this.id().v;this.eat(`(`);let r=[];for(;!this.is(`)`);){let e=this.id().v;this.eat(`:`),r.push({name:e,type:this.type()}),this.is(`)`)||this.eat(`,`)}this.eat(`)`);let i=hi;this.is(`->`)&&(this.eat(`->`),i=this.type()),e.push({name:n,params:r,ret:i,body:this.block(),tok:t})}else if(this.is(`const`)){let e=this.eat(`const`),n=this.id().v,r=null;this.is(`:`)&&(this.eat(`:`),r=this.type()),this.eat(`=`);let i=this.expr();this.eat(`;`),t.push({name:n,type:r,init:i,tok:e})}else this.is(`struct`)?this.err(`structs you declare are not in the shader subset yet`,this.cur,`use vectors, or several values`):this.is(`var`)||this.is(`override`)?this.err(`module-level '${this.cur.v}' is not allowed`,this.cur,`use const, or a param (they become uniforms)`):this.err(`expected 'fn' or 'const' at the top level, found '${this.cur.v}'`);return{fns:e,consts:t}}statements(){let e=[];for(;!this.done;)e.push(this.stmt());return e}block(){this.eat(`{`);let e=[];for(;!this.is(`}`);)this.done&&this.err(`missing '}'`),e.push(this.stmt());return this.eat(`}`),e}stmt(){let e=this.cur;if(this.is(`{`))return{s:`block`,body:this.block(),tok:e};if(this.is(`let`)||this.is(`var`)||this.is(`const`)){let e=this.decl();return this.eat(`;`),e}if(this.is(`if`))return this.ifStmt();if(this.is(`for`)){this.eat(`for`),this.eat(`(`);let t=this.is(`;`)?null:this.is(`let`)||this.is(`var`)?this.decl():this.simple();this.eat(`;`);let n=this.is(`;`)?null:this.expr();this.eat(`;`);let r=this.is(`)`)?null:this.simple();return this.eat(`)`),{s:`for`,init:t,cond:n,update:r,body:this.block(),tok:e}}if(this.is(`while`))return this.eat(`while`),{s:`while`,cond:this.expr(),body:this.block(),tok:e};if((this.is(`loop`)||this.is(`switch`))&&this.err(`'${e.v}' is not in the shader subset yet`,e,e.v===`loop`?`use for or while`:`use if / else`),this.is(`break`)||this.is(`continue`)||this.is(`discard`))return this.p++,this.eat(`;`),{s:e.v,tok:e};if(this.is(`return`)){this.eat(`return`);let t=this.is(`;`)?null:this.expr();return this.eat(`;`),{s:`return`,x:t,tok:e}}let t=this.simple();return this.eat(`;`),t}ifStmt(){let e=this.eat(`if`),t=this.expr(),n=this.block(),r=null;return this.is(`else`)&&(this.eat(`else`),r=this.is(`if`)?[this.ifStmt()]:this.block()),{s:`if`,cond:t,then:n,else:r,tok:e}}decl(){let e=this.cur,t=this.id().v,n=this.id(),r=null;this.is(`:`)&&(this.eat(`:`),r=this.type());let i=null;return this.is(`=`)&&(this.eat(`=`),i=this.expr()),!i&&t!==`var`&&this.err(`'${t} ${n.v}' needs a value`,n,`write ${t} ${n.v} = …;`),{s:`decl`,kind:t,name:n.v,type:r,init:i,tok:e}}simple(){let e=this.cur;if(this.is(`_`))return this.eat(`_`),this.eat(`=`),{s:`expr`,x:this.expr(),tok:e};let t=this.postfix();if(this.is(`++`)||this.is(`--`))return{s:`incr`,op:this.toks[this.p++].v,lhs:t,tok:e};let n=this.cur.v;return this.cur.t===`op`&&[`=`,`+=`,`-=`,`*=`,`/=`,`%=`,`&=`,`|=`,`^=`,`<<=`,`>>=`].includes(n)?(this.p++,{s:`assign`,op:n,lhs:t,rhs:this.expr(),tok:e}):(t.e!==`call`&&this.err(`a statement must assign, call, or declare`,e,`did you mean x = …; ?`),{s:`expr`,x:t,tok:e})}expr(e=1){let t=this.unary();for(;;){let n=this.cur,r=n.t===`op`?wi[n.v]:void 0;if(r===void 0||r<e)return t;this.p++;let i=this.expr(r+1);t={e:`binary`,op:n.v,a:t,b:i,tok:n}}}unary(){let e=this.cur;return e.t===`op`&&(e.v===`-`||e.v===`!`||e.v===`~`)?(this.p++,{e:`unary`,op:e.v,x:this.unary(),tok:e}):(e.t===`op`&&(e.v===`&`||e.v===`*`)&&this.err(`pointers are not in the shader subset`,e,`use plain values`),this.postfix())}postfix(){let e=this.primary();for(;;)if(this.is(`.`)){let t=this.eat(`.`);e={e:`member`,of:e,name:this.id().v,tok:t}}else if(this.is(`[`)){let t=this.eat(`[`),n=this.expr();this.eat(`]`),e={e:`index`,of:e,i:n,tok:t}}else return e}primary(){let e=this.cur;if(e.t===`num`)return this.p++,{e:`num`,v:e.v,tok:e};if(this.is(`(`)){this.eat(`(`);let e=this.expr();return this.eat(`)`),e}if(e.t===`id`){if(e.v===`true`||e.v===`false`)return this.p++,{e:`bool`,v:e.v===`true`,tok:e};if(Ti.test(e.v)&&this.toks[this.p+1]?.v===`<`){let t=this.type();return this.eat(`(`),{e:`call`,name:J(t),targs:t,args:this.args(),tok:e}}return this.p++,this.is(`(`)?(this.eat(`(`),{e:`call`,name:e.v,targs:null,args:this.args(),tok:e}):{e:`id`,name:e.v,tok:e}}return this.err(`unexpected '${e.v||`end of code`}'`)}args(){let e=[];for(;!this.is(`)`);)e.push(this.expr()),this.is(`)`)||this.eat(`,`);return this.eat(`)`),e}};const Di={};for(let e of/* @__PURE__ */ `abs.sin.cos.tan.asin.acos.atan.sinh.cosh.tanh.asinh.acosh.atanh.exp.exp2.log.log2.sqrt.floor.ceil.fract.round.trunc.sign.radians.degrees.normalize.fwidth`.split(`.`))Di[e]={glsl:e,rule:`same`,argc:[1]};Object.assign(Di,{inverseSqrt:{glsl:`inversesqrt`,rule:`same`,argc:[1]},dpdx:{glsl:`dFdx`,rule:`same`,argc:[1]},dpdy:{glsl:`dFdy`,rule:`same`,argc:[1]},min:{glsl:`min`,rule:`same`,argc:[2]},max:{glsl:`max`,rule:`same`,argc:[2]},pow:{glsl:`pow`,rule:`same`,argc:[2]},step:{glsl:`step`,rule:`mixlike`,argc:[2]},reflect:{glsl:`reflect`,rule:`same`,argc:[2]},clamp:{glsl:`clamp`,rule:`mixlike`,argc:[3]},mix:{glsl:`mix`,rule:`mixlike`,argc:[3]},smoothstep:{glsl:`smoothstep`,rule:`mixlike`,argc:[3]},dot:{glsl:`dot`,rule:`scalar`,argc:[2]},length:{glsl:`length`,rule:`scalar`,argc:[1]},distance:{glsl:`distance`,rule:`scalar`,argc:[2]},cross:{glsl:`cross`,rule:`cross`,argc:[2]},any:{glsl:`any`,rule:`bool`,argc:[1]},all:{glsl:`all`,rule:`bool`,argc:[1]},select:{glsl:``,rule:`select`,argc:[3]},transpose:{glsl:`transpose`,rule:`transpose`,argc:[1]},determinant:{glsl:`determinant`,rule:`det`,argc:[1]},fma:{glsl:``,rule:`fma`,argc:[3]},saturate:{glsl:``,rule:`saturate`,argc:[1]},atan2:{glsl:`atan`,rule:`atan2`,argc:[2]}});const Oi=new Set(`attribute const uniform varying layout centroid flat smooth noperspective break continue do for while switch case default if else in out inout float int void bool true false invariant discard return mat2 mat3 mat4 mat2x2 mat2x3 mat2x4 mat3x2 mat3x3 mat3x4 mat4x2 mat4x3 mat4x4 vec2 vec3 vec4 ivec2 ivec3 ivec4 bvec2 bvec3 bvec4 uint uvec2 uvec3 uvec4 lowp mediump highp precision sampler2D sampler3D samplerCube sampler2DShadow samplerCubeShadow sampler2DArray sampler2DArrayShadow isampler2D isampler3D isamplerCube isampler2DArray usampler2D usampler3D usamplerCube usampler2DArray struct asm class union enum typedef template this packed resource goto inline noinline volatile public static extern external interface long short double half fixed unsigned superp input output hvec2 hvec3 hvec4 dvec2 dvec3 dvec4 fvec2 fvec3 fvec4 sampler3DRect filter image1D image2D image3D imageCube iimage1D iimage2D iimage3D iimageCube uimage1D uimage2D uimage3D uimageCube sizeof cast namespace using sample patch subroutine common partition active texture texelFetch mod dFdx dFdy inversesqrt lessThan greaterThan equal notEqual not main`.split(` `)),ki=e=>Oi.has(e)||e.startsWith(`gl_`)||e.includes(`__`)?`w_${e}`:e;var Ai=class{scopes=[];fns=/* @__PURE__ */ new Map;retType=null;where;constructor(e,t){this.where=e;let n=/* @__PURE__ */ new Map;for(let[e,r]of Object.entries(t.values??{})){let t=typeof r==`object`&&`type`in r?r:{type:r};n.set(e,{type:typeof t.type==`string`?gi(t.type):t.type,mutable:!!t.mutable,glsl:t.glsl??e})}for(let[e,r]of Object.entries(t.structs??{})){let t={};for(let[e,n]of Object.entries(r))t[e]=typeof n==`string`?gi(n):n;n.set(e,{type:{k:`struct`,name:e,members:t},mutable:!1,glsl:e})}this.scopes.push(n);for(let e of t.functions??[])this.fns.set(e.name,e)}err(e,t,n){throw new Ci(e,t.line,t.col,n,this.where)}lookup(e){for(let t=this.scopes.length-1;t>=0;t--){let n=this.scopes[t].get(e);if(n)return n}}names(){return this.scopes.flatMap(e=>[...e.keys()])}define(e,t,n,r){this.scopes[this.scopes.length-1].has(e)&&this.err(`'${e}' is already declared here`,r,`use another name, or assign to it`),(Di[e]||/^(f32|i32|u32|bool|vec[234][fiu]?|mat[234]x[234]f)$/.test(e))&&this.err(`'${e}' is a built-in name`,r,`use another name`);let i=ki(e);return this.scopes[this.scopes.length-1].set(e,{type:t,mutable:n,glsl:i}),i}sigs(){return[...this.fns.values()]}module(e){let t=[];for(let n of e.consts){let[e,r]=this.expr(n.init),i=n.type??this.concretize(r);this.assignable(i,r,n.tok),t.push(`const ${Y(i)} ${this.define(n.name,i,!1,n.tok)} = ${this.emitAs(n.init,e,r,i)};`)}for(let t of e.fns)(this.fns.has(t.name)||Di[t.name])&&this.err(`function '${t.name}' is declared twice (or is a built-in)`,t.tok,`rename it`),this.fns.set(t.name,{name:t.name,params:t.params.map(e=>e.type),ret:t.ret});for(let n of e.fns)t.push(`${Y(n.ret)} ${ki(n.name)}(${n.params.map(e=>`${Y(e.type)} ${ki(e.name)}`).join(`, `)});`);for(let n of e.fns){this.scopes.push(/* @__PURE__ */ new Map),this.retType=n.ret;let e=n.params.map(e=>`${Y(e.type)} ${this.define(e.name,e.type,!1,n.tok)}`),r=this.block(n.body,1);n.ret.k!==`void`&&!Li(n.body)&&this.err(`function '${n.name}' does not return a ${J(n.ret)} on every path`,n.tok,`add a return at the end`),t.push(`${Y(n.ret)} ${ki(n.name)}(${e.join(`, `)}) {\n${r}}`),this.scopes.pop(),this.retType=null}return t.join(`
`)}statements(e,t=1){return this.block(e,t,!1)}block(e,t,n=!0){n&&this.scopes.push(/* @__PURE__ */ new Map);let r=`  `.repeat(t),i=e.map(e=>this.stmt(e,t).split(`
`).map(e=>r+e).join(`
`)+`
`).join(``);return n&&this.scopes.pop(),i}stmt(e,t){switch(e.s){case`decl`:{let t,n;if(e.init){let[r,i]=this.expr(e.init);t=e.type??this.concretize(i),this.assignable(t,i,e.tok),n=this.emitAs(e.init,r,i,t)}else t=e.type,t||this.err(`'var ${e.name}' needs a type or a value`,e.tok,`write var ${e.name}: f32;`),n=Ii(t);let r=this.define(e.name,t,e.kind===`var`,e.tok);return`${e.kind===`const`?`const `:``}${Y(t)} ${r} = ${n};`}case`assign`:{let[t,n]=this.lvalue(e.lhs),[r,i]=this.expr(e.rhs);if(e.op===`=`)return this.assignable(n,i,e.tok),`${t} = ${this.emitAs(e.rhs,r,i,n)};`;let a=e.op.slice(0,-1),[o,s]=this.binaryOf(a,t,n,this.emitAs(e.rhs,r,i,bi(i,X(n)??`f32`)),bi(i,X(n)??`f32`),e.tok);return this.assignable(n,s,e.tok),`${t} = ${o};`}case`incr`:{let[t,n]=this.lvalue(e.lhs);return(n.k!==`scalar`||n.s===`bool`||n.s===`f32`)&&this.err(`${e.op} works on i32 / u32 only`,e.tok,`write x = x + 1.0;`),`${t}${e.op};`}case`expr`:return`${this.expr(e.x)[0]};`;case`if`:{let n=`if (${this.cond(e.cond,`if`)}) {\n${this.block(e.then,1)}}`;return e.else&&(n+=e.else.length===1&&e.else[0].s===`if`?` else ${this.stmt(e.else[0],t)}`:` else {\n${this.block(e.else,1)}}`),n}case`for`:{this.scopes.push(/* @__PURE__ */ new Map);let t=`for (${e.init?this.stmt(e.init,0).replace(/;$/,``):``}; ${e.cond?this.cond(e.cond,`for`):``}; ${e.update?this.stmt(e.update,0).replace(/;$/,``):``}) {\n${this.block(e.body,1)}}`;return this.scopes.pop(),t}case`while`:return`while (${this.cond(e.cond,`while`)}) {\n${this.block(e.body,1)}}`;case`break`:case`continue`:case`discard`:return`${e.s};`;case`return`:{let t=this.retType;if(!t)return e.x&&this.err(`a hook does not return a value`,e.tok,`assign to the hook's output instead (e.g. color = …;)`),`return;`;if(!e.x)return t.k!==`void`&&this.err(`return needs a ${J(t)}`,e.tok,`return a value`),`return;`;let[n,r]=this.expr(e.x);return this.assignable(t,r,e.tok),`return ${this.emitAs(e.x,n,r,t)};`}case`block`:return`{\n${this.block(e.body,1)}}`}}cond(e,t){let[n,r]=this.expr(e);return(r.k!==`scalar`||r.s!==`bool`)&&this.err(`the ${t} condition is a ${J(r)}, not a bool`,e.tok,`compare it: x > 0.0`),n}lvalue(e){let t=e;for(;t.e===`member`||t.e===`index`;)t=t.of;t.e!==`id`&&this.err(`you can only assign to a variable`,e.tok,`assign to a var`);let n=this.lookup(t.name);return n||this.unknown(t.name,t.tok),n.mutable||this.err(`'${t.name}' cannot be changed (${n.type.k===`struct`?`a uniform`:`a let, const or input`})`,e.tok,`declare it with var, or use a new var`),e.e===`member`&&e.name.length>1&&/^[xyzwrgba]+$/.test(e.name)&&this.err(`assigning to several components at once (.${e.name}) is not allowed in WGSL`,e.tok,`assign the whole vector: v = vec…(…)`),this.expr(e)}unknown(e,t){let n=this.names().concat(Object.keys(Di)).filter(t=>Ri(t,e)<=2).sort((t,n)=>Ri(t,e)-Ri(n,e))[0];return this.err(`unknown name '${e}'`,t,n?`did you mean '${n}'?`:`the names you can use here: ${this.names().join(`, `)||`none`}`)}concretize(e){return vi(e)?di(e.s):e}assignable(e,t,n){_i(e,t)||(vi(t)&&e.k===`scalar`&&e.s!==`bool`&&t.s!==`bool`?t.s===`f32`&&e.s!==`f32`&&this.err(`a float literal cannot become ${e.s}`,n,`write an integer`):this.err(`expected ${J(e)}, got ${J(t)}`,n,t.k===`scalar`&&e.k===`vec`?`write ${J(e)}(x)`:`convert it: ${J(e)}(…)`))}emitAs(e,t,n,r){return vi(n)?Ni(e,X(r)??`f32`):t}expr(e){switch(e.e){case`num`:return Mi(e);case`bool`:return[String(e.v),mi];case`id`:{let t=this.lookup(e.name);return t||this.unknown(e.name,e.tok),[t.glsl,t.type]}case`member`:{let[t,n]=this.expr(e.of);if(n.k===`struct`){let r=n.members[e.name];return r||this.err(`'${n.name}' has no '${e.name}'`,e.tok,`it has: ${Object.keys(n.members).join(`, `)}`),[`${t}.${e.name}`,r]}return n.k===`vec`?(/^([xyzw]{1,4}|[rgba]{1,4})$/.test(e.name)||this.err(`'.${e.name}' is not a swizzle`,e.tok,`use x y z w or r g b a, up to four`),[...e.name].map(e=>`xyzwrgba`.indexOf(e)%4).some(e=>e>=n.n)&&this.err(`a vec${n.n} has no '.${e.name}'`,e.tok,`use up to ${`xyzw`[n.n-1]}`),[`${Z(t)}.${e.name}`,e.name.length===1?di(n.s):fi(e.name.length,n.s)]):this.err(`a ${J(n)} has no members`,e.tok,`check the name before the dot`)}case`index`:{let[t,n]=this.expr(e.of),[r,i]=this.expr(e.i);(i.k!==`scalar`||i.s!==`i32`&&i.s!==`u32`)&&this.err(`an index must be an integer`,e.i.tok,`write i32(x)`);let a=vi(i)?Ni(e.i,`i32`):r;return n.k===`vec`?[`${Z(t)}[${a}]`,di(n.s)]:n.k===`mat`?[`${Z(t)}[${a}]`,fi(n.r)]:n.k===`array`?[`${Z(t)}[${a}]`,n.of]:this.err(`a ${J(n)} cannot be indexed`,e.tok,`index vectors, matrices or arrays`)}case`unary`:{let[t,n]=this.expr(e.x);return e.op===`-`?((X(n)===`bool`||X(n)===`u32`)&&this.err(`cannot negate a ${J(n)}`,e.tok,`use a signed type`),[`-${Z(t)}`,n]):e.op===`!`?(X(n)!==`bool`&&this.err(`! needs a bool, got ${J(n)}`,e.tok,`compare first`),[n.k===`vec`?`not(${t})`:`!${Z(t)}`,n]):(X(n)!==`i32`&&X(n)!==`u32`&&this.err(`~ needs an integer`,e.tok,`use i32 or u32`),[`~${Z(t)}`,n])}case`binary`:{let[t,n]=this.expr(e.a),[r,i]=this.expr(e.b),a=X(n),o=X(i),s=vi(n)&&!vi(i)?bi(n,o):n,c=vi(i)&&!vi(n)?bi(i,a):i,l=s===n?t:Ni(e.a,o),u=c===i?r:Ni(e.b,a);if(vi(n)&&vi(i)){let n=a===`f32`||o===`f32`,[i,s]=this.binaryOf(e.op,n?Ni(e.a,`f32`):t,di(n?`f32`:a),n?Ni(e.b,`f32`):r,di(n?`f32`:o),e.tok);return[i,!n&&[`+`,`-`,`*`].includes(e.op)&&s.k===`scalar`&&s.s===`i32`?di(`i32`,!0):s]}return this.binaryOf(e.op,l,s,u,c,e.tok)}case`call`:return this.call(e)}}binaryOf(e,t,n,r,i,a){let o=X(n),s=X(i);if(e===`&&`||e===`||`)return(n.k!==`scalar`||n.s!==`bool`||i.k!==`scalar`||i.s!==`bool`)&&this.err(`${e} needs two bools`,a,`compare first; for vectors of bools use all() / any() / select()`),[`${Z(t)} ${e} ${Z(r)}`,mi];if([`==`,`!=`,`<`,`<=`,`>`,`>=`].includes(e))return!_i(n,i)&&(n.k!==`scalar`||i.k!==`scalar`||o!==s)&&this.err(`cannot compare ${J(n)} with ${J(i)}`,a,`convert one side`),n.k===`vec`?[`${{"==":`equal`,"!=":`notEqual`,"<":`lessThan`,"<=":`lessThanEqual`,">":`greaterThan`,">=":`greaterThanEqual`}[e]}(${t}, ${r})`,fi(n.n,`bool`)]:[`${Z(t)} ${e} ${Z(r)}`,mi];if(o===`bool`||s===`bool`){if([`&`,`|`,`^`].includes(e)&&n.k===`scalar`&&i.k===`scalar`)return[`${Z(t)} ${e===`&`?`&&`:e===`|`?`||`:`^^`} ${Z(r)}`,mi];this.err(`${e} does not work on bools`,a,`use && or ||`)}if([`&`,`|`,`^`,`<<`,`>>`].includes(e))return(o!==`i32`&&o!==`u32`||s!==`i32`&&s!==`u32`)&&this.err(`${e} needs integers`,a,`use i32 or u32`),[`${Z(t)} ${e} ${Z(r)}`,n.k===`vec`?n:i.k===`vec`&&e!==`<<`&&e!==`>>`?i:n];o!==s&&this.err(`cannot mix ${J(n)} and ${J(i)} in ${e}`,a,`convert one: ${o===`f32`?`f32(…)`:`${o}(…)`}`);let c;return n.k===`mat`||i.k===`mat`?(e!==`*`&&!(e!==`/`&&e!==`%`&&_i(n,i))&&this.err(`${e} is not defined for matrices here`,a,`use * (matrix × vector, matrix × matrix)`),n.k===`mat`&&i.k===`vec`?(n.c!==i.n&&this.err(`mat${n.c}x${n.r} × vec${i.n}: sizes differ`,a,`match the sizes`),c=fi(n.r)):n.k===`vec`&&i.k===`mat`?(i.r!==n.n&&this.err(`vector × matrix: sizes differ`,a,`match the sizes`),c=fi(i.c)):n.k===`mat`&&i.k===`mat`?(n.c!==i.r&&this.err(`matrix × matrix: sizes differ`,a,`match the sizes`),c={k:`mat`,c:i.c,r:n.r}):c=n.k===`mat`?n:i):n.k===`vec`&&i.k===`vec`?(n.n!==i.n&&this.err(`cannot ${e} a vec${n.n} and a vec${i.n}`,a,`use a swizzle to match sizes (.xyz)`),c=n):c=n.k===`vec`?n:i.k===`vec`?i:vi(n)&&vi(i)?n:di(o),e===`%`&&o===`f32`?[`(${t} - ${Z(r)} * trunc(${Z(t)} / ${Z(r)}))`,c]:[`${Z(t)} ${e} ${Z(r)}`,c]}call(e){let t=e.name,n=e.targs??ji(t);if(n){(n.k===`struct`||n.k===`void`)&&this.err(`cannot construct ${t}`,e.tok,`use a vector or scalar`);let r=n.k===`array`?X(n.of)??`f32`:X(n)??`f32`,i=e.args.map(e=>{let[t,i]=this.expr(e);return vi(i)?[Ni(e,n.k===`scalar`?i.s===`f32`&&n.s!==`f32`?`f32`:n.s:r),di(r)]:[t,i]});if(n.k===`scalar`)return i.length!==1&&this.err(`${t}(…) takes one value`,e.tok,`write ${t}(x)`),i[0][1].k!==`scalar`&&this.err(`${t}(…) takes a scalar, got ${J(i[0][1])}`,e.tok,`pick a component: v.x`),[`${Y(n)}(${i[0][0]})`,n];if(!i.length)return[Ii(n),n];if(n.k===`vec`){let r=i.reduce((e,[,t])=>e+(t.k===`scalar`?1:t.k===`vec`?t.n:99),0);r===n.n||i.length===1&&i[0][1].k===`scalar`||this.err(`${t}(…) needs ${n.n} components, got ${r}`,e.tok,`give ${n.n} numbers, or one to fill all`)}return n.k===`array`&&i.length!==n.n&&this.err(`${t}(…) needs ${n.n} values, got ${i.length}`,e.tok,`give ${n.n}`),[`${Y(n)}(${i.map(e=>e[0]).join(`, `)})`,n]}let r=this.fns.get(t);if(r){r.params.length!==e.args.length&&this.err(`${t}(…) takes ${r.params.length} values, got ${e.args.length}`,e.tok,`match the function`);let n=e.args.map((e,t)=>{let[n,i]=this.expr(e);return this.assignable(r.params[t],i,e.tok),this.emitAs(e,n,i,r.params[t])});return[`${ki(t)}(${n.join(`, `)})`,r.ret]}let i=Di[t];if(!i)return/^texture/.test(t)&&this.err(`'${t}': textures are not in the shader subset yet`,e.tok,`use the inputs the hook gives`),this.unknown(t,e.tok);i.argc.includes(e.args.length)||this.err(`${t}(…) takes ${i.argc.join(` or `)} values, got ${e.args.length}`,e.tok,`see the WGSL built-in ${t}`);let a=e.args.map(e=>this.expr(e)),o=a.find(([,e])=>!vi(e)),s=a.some(([,e])=>X(e)===`f32`)?`f32`:o?X(o[1]):`f32`,c=a.map(([t,n],r)=>vi(n)?[Ni(e.args[r],s),di(s)]:[t,n]),l=c.map(e=>e[1]),u=c.map(e=>e[0]),d=n=>this.err(`${t}(${l.map(J).join(`, `)}): ${n}`,e.tok,`see the WGSL built-in ${t}`),f=l.find(e=>e.k===`vec`)??l[0];switch(i.rule){case`same`:return l.some(e=>e.k!==`scalar`&&e.k!==`vec`)&&d(`needs scalars or vectors`),l.some(e=>!_i(e,l[0]))&&d(`all values must be the same type`),![`abs`,`min`,`max`,`sign`].includes(t)&&X(l[0])!==`f32`&&d(`needs floats`),[`${i.glsl}(${u.join(`, `)})`,l[0]];case`mixlike`:{l.some(e=>e.k!==`scalar`&&e.k!==`vec`)&&d(`needs scalars or vectors`),l.some(e=>X(e)!==X(f))&&d(`mixed scalar types`),l.some(e=>e.k===`vec`&&!_i(e,f))&&d(`vectors of different sizes`),t!==`clamp`&&X(f)!==`f32`&&d(`needs floats`);let e=u.map((e,n)=>l[n].k===`scalar`&&f.k===`vec`&&(t!==`mix`||n!==2)&&!(t===`smoothstep`&&n<2)&&(t!==`step`||n!==0)&&!(t===`clamp`&&n>0)?`${Y(f)}(${e})`:e);return[`${i.glsl}(${e.join(`, `)})`,f]}case`scalar`:return l.some(e=>!yi(e)||e.k===`mat`)&&d(`needs float vectors`),[`${i.glsl}(${u.join(`, `)})`,pi];case`cross`:return l.every(e=>e.k===`vec`&&e.n===3&&e.s===`f32`)||d(`needs two vec3f`),[`cross(${u.join(`, `)})`,fi(3)];case`bool`:return X(l[0])!==`bool`&&d(`needs bools`),[`${i.glsl}(${u[0]})`,mi];case`select`:{let[e,t,n]=l;return _i(e,t)||d(`the first two values must be the same type`),n.k===`scalar`&&n.s===`bool`?[`(${u[2]} ? ${u[1]} : ${u[0]})`,e]:n.k===`vec`&&n.s===`bool`&&e.k===`vec`&&e.n===n.n?[`mix(${u[0]}, ${u[1]}, ${u[2]})`,e]:d(`the condition must be a bool (or a vector of bools of the same size)`)}case`transpose`:return l[0].k!==`mat`&&d(`needs a matrix`),[`transpose(${u[0]})`,{k:`mat`,c:l[0].r,r:l[0].c}];case`det`:return l[0].k!==`mat`&&d(`needs a square matrix`),[`determinant(${u[0]})`,pi];case`fma`:return l.some(e=>!_i(e,l[0])||!yi(e))&&d(`needs three floats of one type`),[`(${Z(u[0])} * ${Z(u[1])} + ${Z(u[2])})`,l[0]];case`saturate`:return yi(l[0])||d(`needs floats`),[`clamp(${u[0]}, ${Y(l[0])}(0.0), ${Y(l[0])}(1.0))`,l[0]];case`atan2`:return(!_i(l[0],l[1])||!yi(l[0]))&&d(`needs two floats of one type`),[`atan(${u[0]}, ${u[1]})`,l[0]];case`bitcast`:return d(`not in the subset`)}}};function ji(e){if(e===`f32`||e===`i32`||e===`u32`||e===`bool`)return di(e);let t=/^vec([234])([fiu])$/.exec(e);return t?fi(+t[1],{f:`f32`,i:`i32`,u:`u32`}[t[2]]):(t=/^mat([234])x([234])f$/.exec(e),t?{k:`mat`,c:+t[1],r:+t[2]}:null)}function Mi(e){let t=e.v;if(/[uU]$/.test(t))return[`${/^0x/i.test(t)?t.slice(0,-1):String(parseInt(t,10))}u`,di(`u32`)];if(/i$/.test(t))return[t.slice(0,-1),di(`i32`)];if(/h$/.test(t))throw new Ci(`'${t}': 16-bit floats are not in the shader subset`,e.tok.line,e.tok.col,`drop the h`);return/f$/.test(t)&&!/^0x/i.test(t)?[Pi(t.slice(0,-1)),pi]:/^0x/i.test(t)?[t,di(`i32`,!0)]:/[.eE]/.test(t)?[Pi(t),di(`f32`,!0)]:[t,di(`i32`,!0)]}function Ni(e,t){if(e.e===`unary`&&e.op===`-`)return`-${Ni(e.x,t)}`;if(e.e!==`num`)return e.e===`binary`?`(${Ni(e.a,t)} ${e.op} ${Ni(e.b,t)})`:`0`;let n=e.v.replace(/[fiu]$/,``),r=/^0x/i.test(n)?parseInt(n,16):Number(n);return t===`f32`?Pi(String(r)):t===`u32`?`${Math.trunc(r)}u`:String(Math.trunc(r))}const Pi=e=>/[.eE]/.test(e)?e.startsWith(`.`)?`0${e}`:e.endsWith(`.`)?`${e}0`:e:`${e}.0`,Z=e=>/^[\w.[\]]+$/.test(e)||/^[\w]+\(.*\)$/.test(e)&&Fi(e)?e:`(${e})`;function Fi(e){let t=0;for(let n=0;n<e.length;n++)if(e[n]===`(`)t++;else if(e[n]===`)`&&(t--,t===0&&n<e.length-1))return!1;return t===0}function Ii(e){switch(e.k){case`scalar`:return e.s===`f32`?`0.0`:e.s===`u32`?`0u`:e.s===`bool`?`false`:`0`;case`vec`:return`${Y(e)}(${Ii(di(e.s))})`;case`mat`:return`${Y(e)}(0.0)`;case`array`:return`${Y(e)}(${Array.from({length:e.n},()=>Ii(e.of)).join(`, `)})`;default:return`0`}}function Li(e){let t=e[e.length-1];return t?t.s===`return`||t.s===`discard`?!0:t.s===`if`?!!t.else&&Li(t.then)&&Li(t.else):t.s===`block`&&Li(t.body):!1}function Ri(e,t){let n=Array.from({length:e.length+1},(e,n)=>[n,...Array(t.length).fill(0)]);for(let e=1;e<=t.length;e++)n[0][e]=e;for(let r=1;r<=e.length;r++)for(let i=1;i<=t.length;i++)n[r][i]=Math.min(n[r-1][i]+1,n[r][i-1]+1,n[r-1][i-1]+(e[r-1]===t[i-1]?0:1));return n[e.length][t.length]}function zi(e,t,n={}){let r=new Ei(Si(e,n.line??0),n.where??``).statements(),i=new Ai(n.where??``,t);return{glsl:i.statements(r,n.indent??1),functions:i.sigs()}}function Bi(e,t={},n={}){let r=new Ei(Si(e,n.line??0),n.where??``).module(),i=new Ai(n.where??``,t);return{glsl:i.module(r),functions:i.sigs().filter(e=>r.fns.some(t=>t.name===e.name))}}function Vi(e){let t=0,n={};for(let[r,i]of e){let[e,a]=i.k===`scalar`?[4,4]:i.k===`vec`?[i.n===2?8:16,i.n*4]:i.k===`mat`&&i.c===4&&i.r===4?[16,64]:[0,0];if(!e)throw new W(`E-SHADER-001`,`a param of type ${J(i)} cannot be a uniform yet`,{where:r,fix:`use f32, i32, u32, vec2f, vec3f, vec4f or mat4x4f`});t=Math.ceil(t/e)*e,n[r]=t,t+=a}return{offsets:n,size:Math.max(16,Math.ceil(t/16)*16)}}const Q=(e,t,n=!1)=>({type:e,doc:t,mutable:n}),Hi={surface:{position:{stage:`vertex`,doc:`move a point of the ground`,values:{position:Q(`vec3f`,`the point, world metres from the view centre (y up)`,!0),uv:Q(`vec2f`,`where in its tile, 0..1`),value:Q(`f32`,`its height, metres × exaggeration`),time:Q(`f32`,`the world's clock, s`)}},color:{stage:`fragment`,doc:`recolour the ground`,values:{color:Q(`vec4f`,`the colour so far (imagery, shade, grade, fog)`,!0),position:Q(`vec3f`,`world metres`),normal:Q(`vec3f`,`the ground normal`),uv:Q(`vec2f`,`where in its tile, 0..1`),value:Q(`f32`,`height, metres`),time:Q(`f32`,`the world's clock, s`)}}},points:{position:{stage:`vertex`,doc:`move or resize a point`,values:{position:Q(`vec3f`,`world metres from the view centre`,!0),size:Q(`f32`,`diameter, CSS px`,!0),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},color:{stage:`fragment`,doc:`recolour a point`,values:{color:Q(`vec4f`,`the colour so far (alpha: the disc edge)`,!0),uv:Q(`vec2f`,`across the disc, -1..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`),position:Q(`vec3f`,`world metres`)}},alpha:{stage:`fragment`,doc:`change how opaque it is`,values:{alpha:Q(`f32`,`opacity so far`,!0),color:Q(`vec4f`,`the final colour`),uv:Q(`vec2f`,`across the disc, -1..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},reveal:{stage:`fragment`,default:`alpha = alpha * reveal;`,doc:"how it appears as `when` fades it in (reveal 0 → 1)",values:{alpha:Q(`f32`,`opacity so far`,!0),reveal:Q(`f32`,`how far shown, 0..1`),uv:Q(`vec2f`,`across the disc, -1..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}}},line:{color:{stage:`fragment`,doc:`recolour a line`,values:{color:Q(`vec4f`,`the colour so far`,!0),alpha:Q(`f32`,`opacity so far (the edge, the reveal)`,!0),along:Q(`f32`,`how far along its line, 0..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`),position:Q(`vec3f`,`world metres`)}},alpha:{stage:`fragment`,doc:`change how opaque it is`,values:{alpha:Q(`f32`,`opacity so far`,!0),color:Q(`vec4f`,`the final colour`),along:Q(`f32`,`how far along, 0..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},reveal:{stage:`fragment`,doc:`how it appears (reveal 0 → 1; with mode "custom" nothing else is applied)`,values:{alpha:Q(`f32`,`opacity so far`,!0),reveal:Q(`f32`,`how far shown, 0..1`),along:Q(`f32`,`how far along, 0..1`),position:Q(`vec3f`,`world metres`),time:Q(`f32`,`the world's clock, s`)}}},area:{position:{stage:`vertex`,doc:`move a point of an area`,values:{position:Q(`vec3f`,`world metres from the view centre (y up)`,!0),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},color:{stage:`fragment`,doc:`recolour an area`,values:{color:Q(`vec4f`,`the colour so far`,!0),alpha:Q(`f32`,`opacity so far`,!0),uv:Q(`vec2f`,`across the feature, -1..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`),position:Q(`vec3f`,`world metres`)}},alpha:{stage:`fragment`,doc:`change how opaque it is`,values:{alpha:Q(`f32`,`opacity so far`,!0),color:Q(`vec4f`,`the final colour`),uv:Q(`vec2f`,`across the feature, -1..1`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},reveal:{stage:`fragment`,doc:`how it appears (reveal 0 → 1)`,values:{alpha:Q(`f32`,`opacity so far`,!0),reveal:Q(`f32`,`how far shown, 0..1`),uv:Q(`vec2f`,`across the feature, -1..1`),position:Q(`vec3f`,`world metres`),time:Q(`f32`,`the world's clock, s`)}}},extrusion:{position:{stage:`vertex`,doc:`move a point of a block`,values:{position:Q(`vec3f`,`world metres from the view centre (y up)`,!0),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},color:{stage:`fragment`,doc:`recolour a block`,values:{color:Q(`vec4f`,`the colour so far (lit)`,!0),alpha:Q(`f32`,`opacity so far`,!0),normal:Q(`vec3f`,`east, up, north`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`),position:Q(`vec3f`,`world metres`)}},alpha:{stage:`fragment`,doc:`change how opaque it is`,values:{alpha:Q(`f32`,`opacity so far`,!0),color:Q(`vec4f`,`the final colour`),value:Q(`f32`,`the feature's value`),time:Q(`f32`,`the world's clock, s`)}},reveal:{stage:`fragment`,doc:`how it appears (reveal 0 → 1)`,values:{alpha:Q(`f32`,`opacity so far`,!0),reveal:Q(`f32`,`how far shown, 0..1`),position:Q(`vec3f`,`world metres`),time:Q(`f32`,`the world's clock, s`)}}},text:{color:{stage:`fragment`,doc:`recolour a label`,values:{color:Q(`vec4f`,`the colour so far (text over its halo)`,!0),uv:Q(`vec2f`,`across the label, 0..1`),time:Q(`f32`,`the world's clock, s`)}},alpha:{stage:`fragment`,doc:`change how opaque it is`,values:{alpha:Q(`f32`,`opacity so far`,!0),uv:Q(`vec2f`,`across the label, 0..1`),time:Q(`f32`,`the world's clock, s`)}},reveal:{stage:`fragment`,doc:`how it appears (reveal 0 → 1)`,values:{alpha:Q(`f32`,`opacity so far`,!0),reveal:Q(`f32`,`how far shown, 0..1`),uv:Q(`vec2f`,`across the label, 0..1`),time:Q(`f32`,`the world's clock, s`)}}}};function Ui(e,t){if(typeof e==`number`)return gi(`f32`);if(typeof e==`string`){if(/^#[0-9a-f]{6}$/i.test(e))return gi(`vec3f`);throw new W(`E-WORLD-004`,`param '${t}': '${e}' is not a number, a vector or a '#rrggbb' colour`,{where:t,fix:`write 1.5, [1, 0.5, 0] or '#ff8800'`})}if(Array.isArray(e)&&e.length>=2&&e.length<=4&&e.every(e=>typeof e==`number`))return gi(`vec${e.length}f`);throw new W(`E-WORLD-004`,`param '${t}' must be a number, 2 to 4 numbers, or a '#rrggbb' colour`,{where:t,fix:`write 1.5, [1, 0.5, 0] or "#ff8800"`})}const Wi=/^[A-Za-z][A-Za-z0-9_]*$/;function Gi(e){let t=Object.entries(e).map(([e,t])=>{if(!Wi.test(e))throw new W(`E-WORLD-004`,`param name '${e}' cannot be a uniform`,{where:e,fix:`use letters, digits and _ , starting with a letter`});return[e,Ui(t,e)]}),n=t.length?t:[[`pad0`,gi(`f32`)]],{offsets:r,size:i}=Vi(n);return{members:t,size:i,offsets:r,wgsl:n.map(([e,t])=>`${e}: ${J(t)},`).join(` `),glsl:n.map(([e,t])=>`${Y(t)} ${e};`).join(` `),pack(n,a=new Float32Array(i/4)){for(let[i,o]of t){let t=n[i]??e[i],s=r[i]/4,c=typeof t==`number`?[t]:typeof t==`string`?Ki(t):[...t];if(o.k===`vec`&&c.length!==o.n||o.k===`scalar`&&c.length!==1)throw new W(`E-WORLD-004`,`param '${i}' is a ${J(o)}; got ${JSON.stringify(t)}`,{where:i,fix:`give ${o.k===`vec`?`${o.n} numbers`:`a number`}`});a.set(c,s)}return a}}}const Ki=e=>{let t=parseInt(e.slice(1),16);return[(t>>16&255)/255,(t>>8&255)/255,(t&255)/255]},$=(e,t,n)=>e.split(`/*@${t}*/`).join(n),qi=e=>({params:Object.fromEntries(e.members)});function Ji(e,t,n,r,i){let a=e.wgsl,o=e.glsl.vertex,s=e.glsl.fragment;for(let e of Object.keys(n))if(e!==`functions`&&!t[e])throw new W(`E-SHADER-001`,`this layer has no '${e}' hook`,{where:i,fix:`its hooks: ${Object.keys(t).join(`, `)||`none`}; or write a custom() layer`});let c=[],l=``;if(n.functions){let e=Bi(n.functions,{structs:qi(r)},{where:`${i} shader.functions`});c=e.functions,l=e.glsl}a=$(a,`functions`,n.functions??``),o=$(o,`functions`,l),s=$(s,`functions`,l),a=$(a,`params`,r.wgsl),o=$(o,`params`,r.glsl),s=$(s,`params`,r.glsl);for(let[e,l]of Object.entries(t)){let t=n[e]??l.default??``;if(!t.trim()){a=$(a,e,``),l.stage===`vertex`?o=$(o,e,``):s=$(s,e,``);continue}let u=zi(t,{values:Object.fromEntries(Object.entries(l.values).map(([e,t])=>[e,{type:t.type,mutable:t.mutable}])),structs:qi(r),functions:c},{where:`${i} shader.${e}`});a=$(a,e,t),l.stage===`vertex`?o=$(o,e,u.glsl.trim()):s=$(s,e,u.glsl.trim())}return{wgsl:a,glsl:{vertex:o,fragment:s}}}const Yi=/* @__PURE__ */ new Set([`corner`,`position`,`size`,`time`,`color`,`params`,`frame`,`o`,`i`,`u`,`at`,`clip`,`alpha`]),Xi={vertex:{corner:Q(`vec2f`,`the quad corner, -1..1`),position:Q(`vec3f`,`the item's place, world metres`,!0),size:Q(`f32`,`CSS px (screen) or metres (ground)`,!0),time:Q(`f32`,`the world's clock, s`)},fragment:{corner:Q(`vec2f`,`across the quad, -1..1`),time:Q(`f32`,`the world's clock, s`),color:Q(`vec4f`,`what you draw (default white)`,!0)}};function Zi(e,t,n,r){let i=(e,t)=>Object.entries(e).map(([e,n])=>{if(!Wi.test(e)||Yi.has(e))throw new W(`E-SHADER-001`,`${t} name '${e}' is taken or not a plain name`,{where:r,fix:`pick another (taken: ${[...Yi].join(`, `)})`});let i=gi(n);if((i.k!==`scalar`||i.s!==`f32`)&&(i.k!==`vec`||i.s!==`f32`))throw new W(`E-SHADER-001`,`${t} '${e}' must be f32 or vec2f..vec4f (got ${n})`,{where:r,fix:`use floats`});return[e,i]}),a=i(t.attributes,`attribute`),o=i(t.varyings,`varying`);for(let[e]of o)if(t.attributes[e])throw new W(`E-SHADER-001`,`'${e}' is both an attribute and a varying`,{where:r,fix:`rename one`});let s=e=>e.k===`scalar`?`0.0`:`${J(e)}(0.0)`,c=e=>e.k===`scalar`?`0.0`:`${Y(e)}(0.0)`,l=e.wgsl,u=e.glsl.vertex,d=e.glsl.fragment;l=$(l,`attributes`,a.map(([e,t],n)=>`@location(${n+1}) a_${e}: ${J(t)},`).join(` `)),u=$(u,`attributes`,a.map(([e,t],n)=>`layout(location = ${n+1}) in ${Y(t)} a_${e};`).join(`
`)),l=$(l,`varyings`,o.map(([e,t],n)=>`@location(${n+1}) v_${e}: ${J(t)},`).join(` `)),u=$(u,`varyings`,o.map(([e,t])=>`out ${Y(t)} v_${e};`).join(`
`)),d=$(d,`varyings`,o.map(([e,t])=>`in ${Y(t)} v_${e};`).join(`
`)),l=$(l,`inputs`,[...a.map(([e])=>`let ${e} = i.a_${e};`),...o.map(([e,t])=>`var ${e} = ${s(t)};`)].join(`
  `)),u=$(u,`inputs`,[...a.map(([e,t])=>`${Y(t)} ${e} = a_${e};`),...o.map(([e,t])=>`${Y(t)} ${e} = ${c(t)};`)].join(`
  `)),l=$(l,`outputs`,o.map(([e])=>`o.v_${e} = ${e};`).join(`
  `)),u=$(u,`outputs`,o.map(([e])=>`v_${e} = ${e};`).join(`
  `)),l=$(l,`finputs`,o.map(([e])=>`var ${e} = i.v_${e};`).join(`
  `)),d=$(d,`finputs`,o.map(([e,t])=>`${Y(t)} ${e} = v_${e};`).join(`
  `));let f={...Object.fromEntries(Object.entries(Xi.vertex).map(([e,t])=>[e,{type:t.type,mutable:t.mutable}])),...Object.fromEntries(a.map(([e,t])=>[e,{type:t}])),...Object.fromEntries(o.map(([e,t])=>[e,{type:t,mutable:!0}]))},p={...Object.fromEntries(Object.entries(Xi.fragment).map(([e,t])=>[e,{type:t.type,mutable:t.mutable}])),...Object.fromEntries(o.map(([e,t])=>[e,{type:t,mutable:!0}]))},m=Ji({wgsl:l,glsl:{vertex:u,fragment:d}},{},{functions:t.functions},n,r);l=m.wgsl,u=m.glsl.vertex,d=m.glsl.fragment;let h=t.functions?Bi(t.functions,{structs:qi(n)},{where:`${r} functions`}).functions:[];for(let[e,i,a]of[[`vertex`,t.vertex??``,f],[`fragment`,t.fragment??``,p]]){let t=i.trim()?zi(i,{values:a,structs:qi(n),functions:h},{where:`${r} ${e}`}).glsl.trim():``;l=$(l,e,i),e===`vertex`?u=$(u,`vertex`,t):d=$(d,`fragment`,t)}return{wgsl:l,glsl:{vertex:u,fragment:d},attributes:a.map(([e,t])=>({name:e,size:t.k===`vec`?t.n:1}))}}const Qi={vertex:{position:Q(`vec3f`,`the point, world metres from the view centre (y up)`,!0),normal:Q(`vec3f`,`its normal`),time:Q(`f32`,`the world's clock, s`)},fragment:{color:Q(`vec4f`,`the colour so far (lit, fogged; alpha 1, or a ghost's)`,!0),base:Q(`vec3f`,`its own colour`),normal:Q(`vec3f`,`its normal`),position:Q(`vec3f`,`world metres`),id:Q(`f32`,`its id (picking)`),time:Q(`f32`,`the world's clock, s`)}};function $i(e,t,n,r,i=2){let a=/* @__PURE__ */ new Set([...Object.keys(Qi.vertex),...Object.keys(Qi.fragment),`params`,`frame`,`o`,`i`,`t`,`m`,`h`,`n`,`p`,`c`,`s`,`r`,`q`,`mat`,`si`,`g`,`keep`]),o=Object.entries(t.varyings??{}).map(([e,t])=>{if(!Wi.test(e)||a.has(e))throw new W(`E-SHADER-001`,`varying name '${e}' is taken or not a plain name`,{where:r,fix:`pick another (taken: ${[...a].join(`, `)})`});let n=gi(t);if((n.k!==`scalar`||n.s!==`f32`)&&(n.k!==`vec`||n.s!==`f32`))throw new W(`E-SHADER-001`,`varying '${e}' must be f32 or vec2f..vec4f (got ${t})`,{where:r,fix:`use floats`});return[e,n]}),s=e=>e.k===`scalar`?`0.0`:`${J(e)}(0.0)`,c=e=>e.k===`scalar`?`0.0`:`${Y(e)}(0.0)`,l=e.wgsl,u=e.glsl.vertex,d=e.glsl.fragment;l=$(l,`mparams`,`struct MP { ${n.wgsl} }\n@group(0) @binding(${i}) var<uniform> params: MP;`);let f=`layout(std140) uniform MP { ${n.glsl} } params;`;u=$(u,`mparams`,f),d=$(d,`mparams`,f),l=$(l,`mvaryings`,o.map(([e,t],n)=>`@location(${7+n}) v_${e}: ${J(t)},`).join(` `)),u=$(u,`mvaryingsOut`,o.map(([e,t])=>`out ${Y(t)} v_${e};`).join(`
`)),d=$(d,`mvaryingsIn`,o.map(([e,t])=>`in ${Y(t)} v_${e};`).join(`
`)),l=$(l,`mvinputs`,[`let normal = o.n;`,`let time = frame.eye.w;`,...o.map(([e,t])=>`var ${e} = ${s(t)};`)].join(`
  `)),u=$(u,`mvinputs`,[`vec3 normal = v_n;`,`float time = frame.eye.w;`,...o.map(([e,t])=>`${Y(t)} ${e} = ${c(t)};`)].join(`
  `)),l=$(l,`mvoutputs`,o.map(([e])=>`o.v_${e} = ${e};`).join(`
  `)),u=$(u,`mvoutputs`,o.map(([e])=>`v_${e} = ${e};`).join(`
  `)),l=$(l,`mfinputs`,[`let base = i.col;`,`let normal = normalize(i.n);`,`let position = i.world;`,`let id = i.id;`,`let time = frame.eye.w;`,...o.map(([e])=>`var ${e} = i.v_${e};`)].join(`
  `)),d=$(d,`mfinputs`,[`vec3 base = v_col;`,`vec3 normal = normalize(v_n);`,`vec3 position = v_world;`,`float id = v_id;`,`float time = frame.eye.w;`,...o.map(([e,t])=>`${Y(t)} ${e} = v_${e};`)].join(`
  `));let p=[],m=``;if(t.functions){let e=Bi(t.functions,{structs:qi(n)},{where:`${r} functions`});p=e.functions,m=e.glsl}l=$(l,`functions`,t.functions??``),u=$(u,`functions`,m),d=$(d,`functions`,m);let h=e=>({...Object.fromEntries(Object.entries(e).map(([e,t])=>[e,{type:t.type,mutable:t.mutable}])),...Object.fromEntries(o.map(([e,t])=>[e,{type:t,mutable:!0}]))});for(let[e,i,a]of[[`vertex`,t.vertex??``,h(Qi.vertex)],[`fragment`,t.fragment,h(Qi.fragment)]]){let t=i.trim()?zi(i,{values:a,structs:qi(n),functions:p},{where:`${r} ${e}`}).glsl.trim():``;l=$(l,e===`vertex`?`mvertex`:`mfragment`,i),e===`vertex`?u=$(u,`mvertex`,t):d=$(d,`mfragment`,t)}return{wgsl:l,glsl:{vertex:u,fragment:d}}}const ea={vertex:{position:Q(`vec3f`,`the vertex, metres from its copy's place (x east, y up, z south)`,!0),normal:Q(`vec3f`,`its normal`,!0),uv:Q(`vec2f`,`its uv`),color:Q(`vec4f`,`its colour`,!0),time:Q(`f32`,`the world's clock, s`),eye:Q(`vec3f`,`the camera, world metres`),origin:Q(`vec3f`,`this copy's place, world metres (billboards: turn position towards the eye)`)},fragment:{color:Q(`vec4f`,`what you draw (default: the vertex colour)`,!0),corner:Q(`vec2f`,`across a point, -1..1 (0 on triangles and lines)`),uv:Q(`vec2f`,`the uv here`),normal:Q(`vec3f`,`the normal here`),position:Q(`vec3f`,`world metres`),time:Q(`f32`,`the world's clock, s`)}},ta=(e,t,n)=>({name:e,params:t.map(e=>gi(e)),ret:gi(n)});function na(e,t,n,r){let i=/* @__PURE__ */ new Set([...Object.keys(ea.vertex),...Object.keys(ea.fragment),`params`,`frame`,`o`,`i`,`u`,`world`,`clip`,`alpha`,`inst`,`vid`,`lambert`,`cornerOf`]),a=(e,t)=>Object.entries(e).map(([e,n])=>{if(!Wi.test(e)||i.has(e))throw new W(`E-SHADER-001`,`${t} name '${e}' is taken or not a plain name`,{where:r,fix:`pick another (taken: ${[...i].join(`, `)})`});i.add(e);let a=gi(n);if((a.k!==`scalar`||a.s!==`f32`)&&(a.k!==`vec`||a.s!==`f32`))throw new W(`E-SHADER-001`,`${t} '${e}' must be f32 or vec2f..vec4f (got ${n})`,{where:r,fix:`use floats`});return[e,a]}),o=a(t.vertexAttributes,`attribute`),s=a(t.instanceAttributes,`instance attribute`),c=a(t.varyings,`varying`);for(let e of t.textures)if(!Wi.test(e)||i.has(e))throw new W(`E-SHADER-001`,`texture name '${e}' is taken or not a plain name`,{where:r,fix:`pick another`});let l=[...o,...s],u=e=>e.k===`scalar`?`0.0`:`${J(e)}(0.0)`,d=e=>e.k===`scalar`?`0.0`:`${Y(e)}(0.0)`,f=e.wgsl,p=e.glsl.vertex,m=e.glsl.fragment;f=$(f,`attributes`,l.map(([e,t],n)=>`@location(${n+5}) a_${e}: ${J(t)},`).join(` `)),p=$(p,`attributes`,l.map(([e,t],n)=>`layout(location = ${n+5}) in ${Y(t)} a_${e};`).join(`
`)),f=$(f,`varyings`,c.map(([e,t],n)=>`@location(${n+6}) v_${e}: ${J(t)},`).join(` `)),p=$(p,`varyings`,c.map(([e,t])=>`out ${Y(t)} v_${e};`).join(`
`)),m=$(m,`varyings`,c.map(([e,t])=>`in ${Y(t)} v_${e};`).join(`
`));let h=t.textures;f=$(f,`textures`,h.length?[...h.map((e,t)=>`@group(1) @binding(${t}) var t_${e}: texture_2d<f32>;`),`@group(1) @binding(${h.length}) var t_sampler: sampler;`,...h.map(e=>`fn tex_${e}(uv: vec2f) -> vec4f {\n  return textureSampleLevel(t_${e}, t_sampler, uv, 0.0);\n}`)].join(`
`):``);let g=h.map(e=>`uniform highp sampler2D t_${e};\nvec4 tex_${e}(vec2 uv) {\n  return textureLod(t_${e}, uv, 0.0);\n}`).join(`
`);p=$(p,`textures`,g),m=$(m,`textures`,g),f=$(f,`inputs`,[...l.map(([e])=>`let ${e} = i.a_${e};`),...c.map(([e,t])=>`var ${e} = ${u(t)};`)].join(`
  `)),p=$(p,`inputs`,[...l.map(([e,t])=>`${Y(t)} ${e} = a_${e};`),...c.map(([e,t])=>`${Y(t)} ${e} = ${d(t)};`)].join(`
  `)),f=$(f,`outputs`,c.map(([e])=>`o.v_${e} = ${e};`).join(`
  `)),p=$(p,`outputs`,c.map(([e])=>`v_${e} = ${e};`).join(`
  `)),f=$(f,`finputs`,c.map(([e])=>`var ${e} = i.v_${e};`).join(`
  `)),m=$(m,`finputs`,c.map(([e,t])=>`${Y(t)} ${e} = v_${e};`).join(`
  `)),f=$(f,`params`,n.wgsl),p=$(p,`params`,n.glsl),m=$(m,`params`,n.glsl);let _=[ta(`lambert`,[`vec3f`],`vec3f`),...h.map(e=>ta(`tex_${e}`,[`vec2f`],`vec4f`))],v=[..._],y=``;if(t.functions){let e=Bi(t.functions,{structs:qi(n),functions:_},{where:`${r} functions`});v=[..._,...e.functions],y=e.glsl}f=$(f,`functions`,t.functions??``),p=$(p,`functions`,y),m=$(m,`functions`,y);let b=(e,t)=>({...Object.fromEntries(Object.entries(e).map(([e,t])=>[e,{type:t.type,mutable:t.mutable}])),...t?Object.fromEntries(l.map(([e,t])=>[e,{type:t}])):{},...Object.fromEntries(c.map(([e,t])=>[e,{type:t,mutable:!0}]))});for(let[e,i,a]of[[`vertex`,t.vertex??``,b(ea.vertex,!0)],[`fragment`,t.fragment??``,b(ea.fragment,!1)]]){let t=i.trim()?zi(i,{values:a,structs:qi(n),functions:v},{where:`${r} ${e}`}).glsl.trim():``;f=$(f,e,i),e===`vertex`?p=$(p,`vertex`,t):m=$(m,`fragment`,t)}let x=e=>e.k===`vec`?e.n:1;return{wgsl:f,glsl:{vertex:p,fragment:m},vertexAttrs:o.map(([e,t])=>({name:e,size:x(t)})),instanceAttrs:s.map(([e,t])=>({name:e,size:x(t)}))}}const ra=/* @__PURE__ */ new WeakMap;function ia(e){let t=/* @__PURE__ */ new Set,n=e=>{if(e instanceof Ur&&!t.has(e)){t.add(e);for(let t of e.inputs())n(t)}},r=e;return n(r.data),n(r.drive?.source),t}let aa=0;const oa=/* @__PURE__ */ new Set([`id`,`when`]),sa=ye.map(e=>e.name);function ca(e){let t=Object.keys(e.level),n=t[0],r=1/0;for(let e of t){let t=sa.indexOf(e);t>=0&&t<r&&(r=t,n=e)}return n===void 0?void 0:e.level[n]}var la=class{id;props;added=-1;shown=0;target=0;paramBlock=null;paramValues={};paramsDirty=!0;failure=null;forced=null;pickNo=0;once=null;detailPinned=!1;detailOwn=1;detailOf(e){return(this.detailPinned?1:e.render.detail??1)*this.detailOwn}attached=null;sourcesInUse=/* @__PURE__ */ new Set;byLevel={};levelRung=null;ownSetProps;constructor(e){let t=e,n=e;for(let r of Object.keys(t)){let i=t[r];!oa.has(r)&&mt(i)&&(n===e&&(n={...t}),this.byLevel[r]=i,n[r]=ca(i))}this.props=n,this.ownSetProps=this.setProps,Object.defineProperty(this,"setProps",{configurable:!0,writable:!0,enumerable:!1,value:e=>{let t=this.byLevel,n={...t};try{this.ownSetProps.call(this,this.leveled(e))}catch(e){for(let e of Object.keys(t))delete t[e];throw Object.assign(t,n),e}return this}});let r=n.id??`${this.constructor.name.toLowerCase()}-${++aa}`;if(!/^[A-Za-z0-9_-]+$/.test(r))throw new W(`E-WORLD-001`,`layer id '${r}' is not a plain name`,{fix:`use letters, digits, '-' and '_' (no dots: params are '<id>.<name>')`});this.id=r,n.params&&(this.paramBlock=Gi(n.params),this.paramValues={...n.params})}atLevel(e,t){let n=this.byLevel,r=Object.keys(n),i=this.props,a={};for(let o of r){let r=ht(n[o],e,t,`${this.id}.${o}`);Object.is(r,i[o])||(a[o]=r)}this.levelRung={names:e,index:t};let o=Object.keys(a);return o.length&&this.ownSetProps.call(this,a),o}get levelNow(){let e=this.levelRung;return e?e.names[e.index]??null:null}leveled(e){let t=e,n=this.levelRung,r=this.byLevel,i=null,a=[];for(let e of Object.keys(t)){let r=t[e];!oa.has(e)&&mt(r)&&(i??={...t},i[e]=n?ht(r,n.names,n.index,`${this.id}.${e}`):ca(r),a.push([e,r]))}for(let e of Object.keys(t))e in r&&delete r[e];for(let[e,t]of a)r[e]=t;return i??t}get order(){return this.props.order}get fades(){return!1}get sits(){return null}get above(){return this.props.above}get below(){return this.props.below}params(){let e={};for(let[t,n]of Object.entries(this.props.params??{}))e[t]={type:typeof n==`number`?`number`:typeof n==`string`?`color`:`vector`,default:n,get:()=>this.paramValues[t],set:e=>this.setParam(t,e)};return e}setParam(e,t){if(!this.props.params||!(e in this.props.params))throw new W(`E-WORLD-004`,`layer '${this.id}' has no param '${e}'`,{where:this.id,fix:`its params: ${Object.keys(this.props.params??{}).join(`, `)||`none`}`});this.paramBlock?.pack({...this.paramValues,[e]:t}),this.paramValues[e]=t,this.paramsDirty=!0}revealSeconds(){return this.props.transition??.35}pickInfo(e){return null}hovered(e){}placeOf(e){return null}owner;animating(e){return!1}wantsFrames(e){return this.animating(e)?1/0:0}readouts(){return{}}points(){return{}}async attach(e){this.attached=e,this.keepSources(ia(this.props))}prepare(e){return!0}status(){return{ready:!0,pending:[],failed:this.failure?[{what:this.id,error:this.failure.message}]:[],missing:[]}}keepSources(e){for(let t of e){let e=ra.get(t);e||ra.set(t,e=/* @__PURE__ */ new Set),e.size||t.reopen(),e.add(this)}let t=this.sourcesInUse;this.sourcesInUse=e;for(let n of t)if(!e.has(n)){let e=ra.get(n);e?.delete(this),e?.size||n.close()}}detach(){this.attached=null,this.keepSources(/* @__PURE__ */ new Set)}setProps(e){this.props={...this.props,...e},this.attached&&this.keepSources(ia(this.props));let t=this.attached?.world;return t?.invalidate?.(),t?.elementChanged?.(this,Object.keys(e)),this}attribution(){return[]}};const ua=`${Sn}
layout(std140) uniform L { vec4 origin; vec4 look; vec4 reveal; vec4 hover; vec4 hover2; vec4 sel; vec4 sel2; vec4 a; vec4 b; vec4 c; vec4 ground; vec4 slots; vec4 slots2; vec4 k0; vec4 k3; vec4 size; vec4 slots3; vec4 k6; } u;
uniform highp sampler2D feat;
const int FW = 512;
const float PI = 3.141592653589793;
const float RE = 6378137.0;
float s2(float a) {
  float s = sin(a * 0.5);
  return 2.0 * s * s;
}
vec3 placeAt(vec2 rel, float h) {
  float dl = u.origin.x + rel.x;
  float dm = u.origin.y + rel.y;
  float c = RE * frame.misc.y;
  vec3 F = vec3(c * dl, h, -c * dm);
  if (frame.size.w >= 1.0) {
    return F;
  }
  float lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  float dp = lat - frame.misc.x;
  float rr = RE + h;
  float cl = cos(lat);
  float k = s2(dp) + frame.misc.y * cl * s2(dl);
  vec3 G = vec3(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + frame.geo.y * cl * s2(dl)));
  return mix(G, F, frame.size.w);
}
void placeUp(vec2 rel, out vec3 p, out vec3 d) {
  float dl = u.origin.x + rel.x;
  float dm = u.origin.y + rel.y;
  float c = RE * frame.misc.y;
  vec3 F = vec3(c * dl, 0.0, -c * dm);
  if (frame.size.w >= 1.0) {
    p = F;
    d = vec3(0.0, 1.0, 0.0);
    return;
  }
  float lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  float dp = lat - frame.misc.x;
  float cl = cos(lat);
  float k = s2(dp) + frame.misc.y * cl * s2(dl);
  vec3 D = vec3(cl * sin(dl), -k, -(sin(dp) + frame.geo.y * cl * s2(dl)));
  p = mix(RE * D, F, frame.size.w);
  d = mix(D + vec3(0.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0), frame.size.w);
}
vec3 liftToEye(vec3 p, float px) {
  vec3 toEye = frame.eye.xyz - p;
  float d = length(toEye);
  float mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(px * frame.size.z * mpp, d * 0.5);
}
float mppAt(vec3 p) {
  return 2.0 * length(frame.eye.xyz - p) * frame.misc.z / frame.size.y;
}
float slotOf(int plane) {
  float s = u.slots2.y;
  if (plane == 0) {
    s = u.slots.x;
  }
  if (plane == 1) {
    s = u.slots.y;
  }
  if (plane == 2) {
    s = u.slots2.x;
  }
  if (plane == 3) {
    s = u.slots.z;
  }
  if (plane == 4) {
    s = u.slots.w;
  }
  if (plane == 6) {
    s = u.slots3.x;
  }
  return s;
}
vec4 constOf(int plane) {
  if (plane <= 1) {
    return u.k0;
  }
  if (plane == 3) {
    return u.k3;
  }
  if (plane == 5) {
    return vec4(0.0, 0.0, 0.0, 1.0);
  }
  if (plane == 6) {
    return u.k6;
  }
  return vec4(0.0);
}
vec4 featAt(float index, int plane) {
  float s = slotOf(plane);
  if (s < 0.0) {
    return constOf(plane);
  }
  int i = int(index + 0.5);
  return texelFetch(feat, ivec2(i % FW, i / FW + int(s) * int(u.look.w)), 0);
}
vec4 colourOf(float index, vec4 state) {
  vec4 c = mix(featAt(index, 1), featAt(index, 0), u.look.z);
  c = vec4(mix(c.rgb, u.sel.rgb, u.sel.a * state.y), c.a);
  c = vec4(mix(c.rgb, u.hover.rgb, u.hover.a * state.x), c.a);
  return c;
}
float raiseOf(vec4 state) {
  return u.hover2.x * state.x + u.sel2.x * state.y;
}
bool dashOn(float at, vec4 d) {
  float p = d.x + d.y + d.z + d.w;
  if (d.x <= 0.0 || p <= 0.0001) {
    return true;
  }
  float k = at - floor(at / p) * p;
  return k < d.x || (k >= d.x + d.y && k < d.x + d.y + d.z);
}
float acrossAt(float t, float s, float len, float k, vec4 ends) {
  float cap = u.a.z;
  if (t < 0.0) {
    if (ends.z > 0.5 || (cap > 0.5 && cap < 1.5)) {
      return 1e9;
    }
    if (cap > 1.5) {
      return max(abs(s), -t * len) / k;
    }
    return length(vec2(s, t * len)) / k;
  }
  if (t >= 1.0) {
    if (ends.w > 0.5) {
      if (u.a.w < 0.5) {
        return 1e9;
      }
      return length(vec2(s, (t - 1.0) * len)) / k;
    }
    if (cap > 0.5 && cap < 1.5) {
      return 1e9;
    }
    if (cap > 1.5) {
      return max(abs(s), (t - 1.0) * len) / k;
    }
    return length(vec2(s, (t - 1.0) * len)) / k;
  }
  return s / k;
}
float easeOutBack(float t) {
  float c = 1.70158;
  float x = t - 1.0;
  return 1.0 + (c + 1.0) * x * x * x + c * x * x;
}
`,da=`${ua}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,fa={vertex:`${da}
layout(location = 0) in vec4 v;
out vec4 v_col;
out vec2 v_q;
flat out float v_idx;
flat out vec4 v_state;
out vec3 v_world;
flat out float v_value;
vec2 metresToRel(float e, float n) {
  return vec2(e, n) / (RE * max(frame.misc.y, 0.01));
}
void main() {
  float idx = v.w;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  vec4 box = featAt(idx, 4);
  vec4 nums = featAt(idx, 3);
  vec2 rel = v.xy;
  if (u.reveal.x == 4.0) {
    rel = box.xy + (rel - box.xy) * state.z;
  }
  rel = box.xy + (rel - box.xy) * motion.w + metresToRel(motion.x, motion.y);
  float value = nums.z;
  float time = frame.eye.w;
  float h = v.z * u.origin.z + u.origin.w + raiseOf(state) + motion.z + mix(nums.y, nums.x, u.look.z) * u.a.y;
  vec3 position = placeAt(rel, h);
  /*@position*/
  gl_Position = clipOut(frame.viewProj * vec4(liftToEye(position, u.a.x), 1.0));
  v_col = colourOf(idx, state);
  v_q = (v.xy - box.xy) / max(box.zw, vec2(0.0000001));
  v_idx = idx;
  v_state = state;
  v_world = position;
  v_value = value;
}
`,fragment:`${da}
in vec4 v_col;
in vec2 v_q;
flat in float v_idx;
flat in vec4 v_state;
in vec3 v_world;
flat in float v_value;
out vec4 outColor;
void main() {
  vec4 color = v_col;
  float alpha = color.a;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  float time = frame.eye.w;
  float value = v_value;
  vec3 position = v_world;
  vec2 uv = v_q;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    float e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, v_q.x));
  } else if (mode == 6.0) {
    float e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(v_q)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.hover2.w, v_idx);
    return;
  }
  outColor = vec4(themed(color.rgb, max(v_state.x, v_state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`},pa=`uniform highp sampler2D img;
vec4 imageAt(vec2 uv) {
  vec4 c = textureLod(img, vec2(uv.x * 0.5 + 0.5, 0.5 - uv.y * 0.5), 0.0);
  return vec4(c.rgb / max(c.a, 0.0001), c.a);
}
`,ma=`${In}
// origin: (lon of the origin − the centre's, radians · Mercator y likewise · exaggeration · lift m)
// look: (opacity, shown, transition mix, rows per plane) · reveal: (mode, 1 when plane 5 moves features, soft, glow)
// hover: (rgb, mix) · hover2: (raise m, outline px, outline mix, pick number) · sel: (rgb, mix) · sel2: (raise, outline px, 0, 0)
// a, b, c: the mark's own · ground: (the lowest, the highest ground under it, m) for marks draped on the ground
// slots: (colour, colour before, numbers, box) planes' row blocks · slots2: (state, motion, 0, 0) · k0: the colour and
// k3: the numbers when they are the same for every feature · size: how its size or width is drawn (units, least, most
// CSS px: core › render › drawing › instancing, sizePx) · slots3: (the mark's own plane's row block, 0, 0, 0) · k6: that
// plane when it is the same for every feature
struct L { origin: vec4f, look: vec4f, reveal: vec4f, hover: vec4f, hover2: vec4f, sel: vec4f, sel2: vec4f, a: vec4f, b: vec4f, c: vec4f, ground: vec4f, slots: vec4f, slots2: vec4f, k0: vec4f, k3: vec4f, size: vec4f, slots3: vec4f, k6: vec4f }
@group(0) @binding(1) var<uniform> u: L;
@group(1) @binding(0) var feat: texture_2d<f32>;
const FW = 512;
const PI = 3.141592653589793;
const RE = 6378137.0;
fn s2(a: f32) -> f32 {
  let s = sin(a * 0.5);
  return 2.0 * s * s;
}
// a place → world metres from the view centre (y up), on the globe and flat, by the morph
fn placeAt(rel: vec2f, h: f32) -> vec3f {
  let dl = u.origin.x + rel.x;
  let dm = u.origin.y + rel.y;
  let c = RE * frame.misc.y;
  let F = vec3f(c * dl, h, -c * dm);
  if (frame.size.w >= 1.0) {
    return F;
  }
  let lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  let dp = lat - frame.misc.x;
  let rr = RE + h;
  let cl = cos(lat);
  let k = s2(dp) + frame.misc.y * cl * s2(dl);
  let G = vec3f(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + frame.geo.y * cl * s2(dl)));
  return mix(G, F, frame.size.w);
}
// a place on the ground (p) and how it moves per metre up (d): placeAt(rel, h) = p + h · d, one evaluation for every
// height (the draped marks place a piece's corners at several heights)
struct Up { p: vec3f, d: vec3f }
fn placeUp(rel: vec2f) -> Up {
  let dl = u.origin.x + rel.x;
  let dm = u.origin.y + rel.y;
  let c = RE * frame.misc.y;
  let F = vec3f(c * dl, 0.0, -c * dm);
  if (frame.size.w >= 1.0) {
    return Up(F, vec3f(0.0, 1.0, 0.0));
  }
  let lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  let dp = lat - frame.misc.x;
  let cl = cos(lat);
  let k = s2(dp) + frame.misc.y * cl * s2(dl);
  let D = vec3f(cl * sin(dl), -k, -(sin(dp) + frame.geo.y * cl * s2(dl)));
  return Up(mix(RE * D, F, frame.size.w), mix(D + vec3f(0.0, 1.0, 0.0), vec3f(0.0, 1.0, 0.0), frame.size.w));
}
// moved towards the eye by px pixels' worth of metres: drawn over the ground under it, never into it
fn liftToEye(p: vec3f, px: f32) -> vec3f {
  let toEye = frame.eye.xyz - p;
  let d = length(toEye);
  let mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(px * frame.size.z * mpp, d * 0.5);
}
// metres per device pixel at a place
fn mppAt(p: vec3f) -> f32 {
  return 2.0 * length(frame.eye.xyz - p) * frame.misc.z / frame.size.y;
}
// where a plane is in the texture (its row block; -1: not there)
fn slotOf(plane: i32) -> f32 {
  var s = u.slots2.y;
  if (plane == 0) {
    s = u.slots.x;
  }
  if (plane == 1) {
    s = u.slots.y;
  }
  if (plane == 2) {
    s = u.slots2.x;
  }
  if (plane == 3) {
    s = u.slots.z;
  }
  if (plane == 4) {
    s = u.slots.w;
  }
  if (plane == 6) {
    s = u.slots3.x;
  }
  return s;
}
// a plane the same for every feature: the colour (now and before), the numbers, no motion, no box
fn constOf(plane: i32) -> vec4f {
  if (plane <= 1) {
    return u.k0;
  }
  if (plane == 3) {
    return u.k3;
  }
  if (plane == 5) {
    return vec4f(0.0, 0.0, 0.0, 1.0);
  }
  if (plane == 6) {
    return u.k6;
  }
  return vec4f(0.0);
}
// a feature's texel in a plane
fn featAt(index: f32, plane: i32) -> vec4f {
  let s = slotOf(plane);
  if (s < 0.0) {
    return constOf(plane);
  }
  let i = i32(index + 0.5);
  return textureLoad(feat, vec2i(i % FW, i / FW + i32(s) * i32(u.look.w)), 0);
}
// the colour now: the transition between before and now, then the hover and selection looks
fn colourOf(index: f32, state: vec4f) -> vec4f {
  var c = mix(featAt(index, 1), featAt(index, 0), u.look.z);
  c = vec4f(mix(c.rgb, u.sel.rgb, u.sel.a * state.y), c.a);
  c = vec4f(mix(c.rgb, u.hover.rgb, u.hover.a * state.x), c.a);
  return c;
}
fn raiseOf(state: vec4f) -> f32 {
  return u.hover2.x * state.x + u.sel2.x * state.y;
}
// a dash pattern (on, off, on, off CSS px; the first 0: none) at a distance along it: false in its gaps
fn dashOn(at: f32, d: vec4f) -> bool {
  let p = d.x + d.y + d.z + d.w;
  if (d.x <= 0.0 || p <= 0.0001) {
    return true;
  }
  let k = at - floor(at / p) * p;
  return k < d.x || (k >= d.x + d.y && k < d.x + d.y + d.z);
}
// how far across a line's piece a point is, in px (t: along the piece 0..1, s: across, len: the piece's length, k: units
// a px; ends.z / .w: a piece before / after it): at the line's ends its caps (u.a.z: 0 round, 1 butt, 2 square); a
// joint belongs to the piece after it, and to this piece's round end too with round joins (u.a.w: 1); 1e9: not drawn
fn acrossAt(t: f32, s: f32, len: f32, k: f32, ends: vec4f) -> f32 {
  let cap = u.a.z;
  if (t < 0.0) {
    if (ends.z > 0.5 || (cap > 0.5 && cap < 1.5)) {
      return 1e9;
    }
    if (cap > 1.5) {
      return max(abs(s), -t * len) / k;
    }
    return length(vec2f(s, t * len)) / k;
  }
  if (t >= 1.0) {
    if (ends.w > 0.5) {
      if (u.a.w < 0.5) {
        return 1e9;
      }
      return length(vec2f(s, (t - 1.0) * len)) / k;
    }
    if (cap > 0.5 && cap < 1.5) {
      return 1e9;
    }
    if (cap > 1.5) {
      return max(abs(s), (t - 1.0) * len) / k;
    }
    return length(vec2f(s, (t - 1.0) * len)) / k;
  }
  return s / k;
}
fn easeOutBack(t: f32) -> f32 {
  let c = 1.70158;
  let x = t - 1.0;
  return 1.0 + (c + 1.0) * x * x * x + c * x * x;
}
`,ha=`${ma}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct I { @location(0) v: vec4f }
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec4f,
  @location(1) q: vec2f,
  @location(2) @interpolate(flat) idx: f32,
  @location(3) @interpolate(flat) state: vec4f,
  @location(4) world: vec3f,
  @location(5) @interpolate(flat) value: f32,
}
/*@functions*/
fn metresToRel(e: f32, n: f32) -> vec2f {
  return vec2f(e, n) / (RE * max(frame.misc.y, 0.01));
}
@vertex fn vs(i: I) -> O {
  let idx = i.v.w;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  let box = featAt(idx, 4);
  let nums = featAt(idx, 3);
  var rel = i.v.xy;
  if (u.reveal.x == 4.0) {
    rel = box.xy + (rel - box.xy) * state.z;
  }
  rel = box.xy + (rel - box.xy) * motion.w + metresToRel(motion.x, motion.y);
  let value = nums.z;
  let time = frame.eye.w;
  let h = i.v.z * u.origin.z + u.origin.w + raiseOf(state) + motion.z + mix(nums.y, nums.x, u.look.z) * u.a.y;
  var position = placeAt(rel, h);
  /*@position*/
  var o: O;
  o.pos = clipOut(frame.viewProj * vec4f(liftToEye(position, u.a.x), 1.0));
  o.col = colourOf(idx, state);
  o.q = (i.v.xy - box.xy) / max(box.zw, vec2f(0.0000001));
  o.idx = idx;
  o.state = state;
  o.world = position;
  o.value = value;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  var color = i.col;
  var alpha = color.a;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  let time = frame.eye.w;
  let value = i.value;
  let position = i.world;
  let uv = i.q;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    let e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, i.q.x));
  } else if (mode == 6.0) {
    let e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(i.q)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.hover2.w, i.idx);
  }
  return vec4f(themed(color.rgb, max(i.state.x, i.state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`,ga=e=>`@group(1) @binding(${e}) var img: texture_2d<f32>;
@group(1) @binding(${e+1}) var imgSampler: sampler;
fn imageAt(uv: vec2f) -> vec4f {
  // (held as colours times alpha: filtered without fringes, divided again here)
  let c = textureSampleLevel(img, imgSampler, vec2f(uv.x * 0.5 + 0.5, 0.5 - uv.y * 0.5), 0.0);
  return vec4f(c.rgb / max(c.a, 0.0001), c.a);
}
`;function _a(e,t=[],n=2){let r=t.length>0,i=va(e,0,r?t[0]*n:e.length,n,!0),a=[];return!i||i.next===i.prev||(r&&(i=wa(e,t,i,n)),ba(i,a,0)),a}function va(e,t,n,r,i){let a=null;if(i===Ua(e,t,n,r)>0)for(let i=t;i<n;i+=r)a=Ba(i/r,e[i],e[i+1],a);else for(let i=n-r;i>=t;i-=r)a=Ba(i/r,e[i],e[i+1],a);return a&&Ma(a,a.next)&&(Va(a),a=a.next),a}function ya(e,t){let n=e,r;t??=e;do if(r=!1,!n.steiner&&(Ma(n,n.next)||ja(n.prev,n,n.next)===0)){if(Va(n),n=t=n.prev,n===n.next)break;r=!0}else n=n.next;while(r||n!==t);return t}function ba(e,t,n){if(!e)return;let r=e;for(;e.prev!==e.next;){let i=e.prev,a=e.next;if(xa(e))t.push(i.i,e.i,a.i),Va(e),e=a.next,r=a.next;else if(e=a,e===r){n?n===1?(e=Sa(ya(e),t),ba(e,t,2)):n===2&&Ca(e,t):ba(ya(e),t,1);break}}}function xa(e){let t=e.prev,n=e,r=e.next;if(ja(t,n,r)>=0)return!1;let i=Math.min(t.x,n.x,r.x),a=Math.min(t.y,n.y,r.y),o=Math.max(t.x,n.x,r.x),s=Math.max(t.y,n.y,r.y),c=r.next;for(;c!==t;){if(c.x>=i&&c.x<=o&&c.y>=a&&c.y<=s&&ka(t.x,t.y,n.x,n.y,r.x,r.y,c.x,c.y)&&ja(c.prev,c,c.next)>=0)return!1;c=c.next}return!0}function Sa(e,t){let n=e;do{let r=n.prev,i=n.next.next;!Ma(r,i)&&Na(r,n,n.next,i)&&La(r,i)&&La(i,r)&&(t.push(r.i,n.i,i.i),Va(n),Va(n.next),n=e=i),n=n.next}while(n!==e);return ya(n)}function Ca(e,t){let n=e;do{let e=n.next.next;for(;e!==n.prev;){if(n.i!==e.i&&Aa(n,e)){let r=za(n,e);n=ya(n,n.next),r=ya(r,r.next),ba(n,t,0),ba(r,t,0);return}e=e.next}n=n.next}while(n!==e)}function wa(e,t,n,r){let i=[];for(let n=0;n<t.length;n++){let a=va(e,t[n]*r,n<t.length-1?t[n+1]*r:e.length,r,!1);a&&(a===a.next&&(a.steiner=!0),i.push(Oa(a)))}i.sort((e,t)=>e.x-t.x);for(let e of i)n=Ta(e,n);return n}function Ta(e,t){let n=Ea(e,t);if(!n)return t;let r=za(n,e);return ya(r,r.next),ya(n,n.next)}function Ea(e,t){let n=t,r=-1/0,i=null,a=e.x,o=e.y;do{if(o<=n.y&&o>=n.next.y&&n.next.y!==n.y){let e=n.x+(o-n.y)*(n.next.x-n.x)/(n.next.y-n.y);if(e<=a&&e>r&&(r=e,i=n.x<n.next.x?n:n.next,e===a))return i}n=n.next}while(n!==t);if(!i)return null;let s=i,c=i.x,l=i.y,u=1/0;n=i;do{if(a>=n.x&&n.x>=c&&a!==n.x&&ka(o<l?a:r,o,c,l,o<l?r:a,o,n.x,n.y)){let t=Math.abs(o-n.y)/(a-n.x);La(n,e)&&(t<u||t===u&&(n.x>i.x||n.x===i.x&&Da(i,n)))&&(i=n,u=t)}n=n.next}while(n!==s);return i}const Da=(e,t)=>ja(e.prev,e,t.prev)<0&&ja(t.next,e,e.next)<0;function Oa(e){let t=e,n=e;do(t.x<n.x||t.x===n.x&&t.y<n.y)&&(n=t),t=t.next;while(t!==e);return n}function ka(e,t,n,r,i,a,o,s){return(i-o)*(t-s)>=(e-o)*(a-s)&&(e-o)*(r-s)>=(n-o)*(t-s)&&(n-o)*(a-s)>=(i-o)*(r-s)}function Aa(e,t){return e.next.i!==t.i&&e.prev.i!==t.i&&!Ia(e,t)&&(La(e,t)&&La(t,e)&&Ra(e,t)&&(ja(e.prev,e,t.prev)!==0||ja(e,t.prev,t)!==0)||Ma(e,t)&&ja(e.prev,e,e.next)>0&&ja(t.prev,t,t.next)>0)}const ja=(e,t,n)=>(t.y-e.y)*(n.x-t.x)-(t.x-e.x)*(n.y-t.y),Ma=(e,t)=>e.x===t.x&&e.y===t.y;function Na(e,t,n,r){let i=Fa(ja(e,t,n)),a=Fa(ja(e,t,r)),o=Fa(ja(n,r,e)),s=Fa(ja(n,r,t));return!!(i!==a&&o!==s||i===0&&Pa(e,n,t)||a===0&&Pa(e,r,t)||o===0&&Pa(n,e,r)||s===0&&Pa(n,t,r))}const Pa=(e,t,n)=>t.x<=Math.max(e.x,n.x)&&t.x>=Math.min(e.x,n.x)&&t.y<=Math.max(e.y,n.y)&&t.y>=Math.min(e.y,n.y),Fa=e=>e>0?1:e<0?-1:0;function Ia(e,t){let n=e;do{if(n.i!==e.i&&n.next.i!==e.i&&n.i!==t.i&&n.next.i!==t.i&&Na(n,n.next,e,t))return!0;n=n.next}while(n!==e);return!1}const La=(e,t)=>ja(e.prev,e,e.next)<0?ja(e,t,e.next)>=0&&ja(e,e.prev,t)>=0:ja(e,t,e.prev)<0||ja(e,e.next,t)<0;function Ra(e,t){let n=e,r=!1,i=(e.x+t.x)/2,a=(e.y+t.y)/2;do n.y>a!=n.next.y>a&&n.next.y!==n.y&&i<(n.next.x-n.x)*(a-n.y)/(n.next.y-n.y)+n.x&&(r=!r),n=n.next;while(n!==e);return r}function za(e,t){let n=Ha(e.i,e.x,e.y),r=Ha(t.i,t.x,t.y),i=e.next,a=t.prev;return e.next=t,t.prev=e,n.next=i,i.prev=n,r.next=n,n.prev=r,a.next=r,r.prev=a,r}function Ba(e,t,n,r){let i=Ha(e,t,n);return r?(i.next=r.next,i.prev=r,r.next.prev=i,r.next=i):(i.prev=i,i.next=i),i}function Va(e){e.next.prev=e.prev,e.prev.next=e.next}function Ha(e,t,n){let r={i:e,x:t,y:n,steiner:!1};return r.prev=r,r.next=r,r}function Ua(e,t,n,r){let i=0;for(let a=t,o=n-r;a<n;a+=r)i+=(e[o]-e[a])*(e[a+1]+e[o+1]),o=a;return i}const Wa=85.0511;var Ga=class{lon0;lat0;m0;k;constructor(e,t){this.lon0=e,this.lat0=Math.max(-85,Math.min(85,t)),this.m0=ae(this.lat0);let n=this.lat0*B,r=Math.tan(n),i=1/Math.cos(n),a=r*r;this.k=Math.abs(this.lat0)<70?[i,i*r/2,i*(2*a+1)/6,i*r*(6*a+5)/24,i*(24*a*a+28*a+5)/120]:null}rel(e,t){return[this.x(e),this.y(t)]}lonLat(e,t){return[this.lonOf(e),this.latOf(t)]}x(e){let t=e-this.lon0;return(t>-180&&t<180?t:H(t))*B}y(e){let t=e-this.lat0,n=this.k;if(n&&t>-.5&&t<.5){let e=t*B;return e*(n[0]+e*(n[1]+e*(n[2]+e*(n[3]+e*n[4]))))}return ae(Math.max(-85.0511,Math.min(Wa,e)))-this.m0}lonOf(e){return H(this.lon0+e/B)}latOf(e){return V(this.m0+e)}offset(e,t){return[H(this.lon0-e)*B,this.m0-ae(Math.max(-85.0511,Math.min(Wa,t)))]}};function Ka(e,t,n,r,i){let a=r-t,o=i-n;return Math.sqrt(a*a+o*o)*z/Math.cosh(e.m0+(n+i)/2)}var qa=class{a;n=0;constructor(e){this.a=e}room(e){if(this.n+e>this.a.length){let t=new this.a.constructor(Math.max(this.n+e,this.a.length*2));t.set(this.a.subarray(0,this.n)),this.a=t}return this.a}push(e){this.room(1)[this.n++]=e}push2(e,t){let n=this.room(2);n[this.n]=e,n[this.n+1]=t,this.n+=2}done(){return this.n===this.a.length?this.a:this.a.slice(0,this.n)}clear(){this.n=0}};const Ja=(e,t)=>{for(;e-t>Math.PI;)e-=2*Math.PI;for(;e-t<-Math.PI;)e+=2*Math.PI;return e},Ya=67108864;function Xa(e,t,n,r=10){let i=0,a=new Int32Array(t.length);for(let e=0;e<t.length;e++){let n=t[e],r=n.length>1&&n[0][0]===n[n.length-1][0]&&n[0][1]===n[n.length-1][1]?n.length-1:n.length;i+=r,a[e]=i}let o=new Float64Array(i*2),s=[],c=1/0,l=1/0,u=-1/0,d=-1/0;for(let n=0,r=0;n<t.length;n++){let i=t[n];n>0&&s.push(r);let f=NaN;for(let t=0;r<a[n];t++,r++){let n=e.x(i[t][0]),a=e.y(i[t][1]);f===f&&(n=Ja(n,f)),f=n,o[2*r]=n,o[2*r+1]=a,n<c&&(c=n),n>u&&(u=n),a<l&&(l=a),a>d&&(d=a)}}let f=_a(o,s,2),p=e.m0+l,m=e.m0+d,h=z/(p*m<=0?1:Math.min(Math.cosh(p),Math.cosh(m)));if(!i||Math.hypot(u-c,d-l)*h<=n)return{xy:o,tris:f};let g=new qa(new Float64Array(o.length*2)),_=new qa(new Uint32Array(f.length*2)),v=/* @__PURE__ */ new Map;g.room(o.length).set(o),g.n=o.length;let y=(e,t)=>{let n=e<t?e*Ya+t:t*Ya+e,r=v.get(n);if(r===void 0){let i=g.a;r=g.n/2,g.push2((i[2*e]+i[2*t])/2,(i[2*e+1]+i[2*t+1])/2),v.set(n,r)}return r},b=(t,n)=>{let r=g.a;return Ka(e,r[2*t],r[2*t+1],r[2*n],r[2*n+1])},x=(e,t,i,a)=>{let o=b(e,t),s=b(t,i),c=b(i,e),l=Math.max(o,s,c);if(l<=n||a>=r){let n=_.room(3);n[_.n]=e,n[_.n+1]=t,n[_.n+2]=i,_.n+=3}else if(l===o){let n=y(e,t);x(e,n,i,a+1),x(n,t,i,a+1)}else if(l===s){let n=y(t,i);x(e,t,n,a+1),x(e,n,i,a+1)}else{let n=y(i,e);x(e,t,n,a+1),x(n,t,i,a+1)}};for(let e=0;e<f.length;e+=3)x(f[e],f[e+1],f[e+2],0);return{xy:g.done(),tris:_.done()}}const Za=e=>Math.max(40,Math.min(6e4,e/96)),Qa={fade:0,trace:1,pulse:2,"trace-fill":3,grow:4,wipe:5,radial:6,sweep:7,pop:8,rise:9,ripple:10,type:11,count:12,mask:13,custom:14},$a={line:[`trace`,`pulse`,`fade`],area:[`trace-fill`,`grow`,`wipe`,`radial`,`sweep`,`fade`],extrusion:[`rise`,`pop`,`sweep`,`fade`],point:[`sweep`,`pop`,`fade`],text:[`type`,`count`,`mask`,`fade`],custom:[`fade`]},eo=/* @__PURE__ */ new Map,to={define(e,t){if(!/^[a-z][a-z0-9-]*$/.test(e))throw new W(`E-DATA-001`,`reveal mode name '${e}' is not plain`,{fix:`use lowercase letters, digits and '-', like 'drop-in'`});if(e in Qa)throw new W(`E-DATA-001`,`'${e}' is a built-in mode`,{fix:`pick another name`});if(!t||typeof t.feature!=`function`&&typeof t.wgsl!=`string`)throw new W(`E-DATA-001`,`reveal.define('${e}') needs feature (d, p) => look, or wgsl`,{fix:`reveal.define('drop-in', { feature: (d, p) => ({ opacity: p }) })`});return eo.set(e,t),e},get:e=>eo.get(e),names:()=>[...Object.keys(Qa).filter(e=>e!==`custom`),...eo.keys()],undefine:e=>eo.delete(e)},no=.25;function ro(e,t,n){let r=typeof e==`string`?{mode:e}:e??{},i=n.context===`app`&&r.app===void 0;for(let e of[r.mode,r.video,r.app])if(e!==void 0&&!eo.has(String(e))&&!(String(e)in Qa))throw new W(`E-DATA-001`,`no reveal mode '${e}'`,{fix:`use ${to.names().join(`, `)}, or reveal.define('${e}', …)`});let a=String(i?`fade`:(n.context===`video`?r.video:r.app)??r.mode??`fade`),o=eo.get(a)??null,s=o?o.base??(o.wgsl?`custom`:`fade`):a;if(!o&&!(a in Qa))throw new W(`E-DATA-001`,`no reveal mode '${a}'`,{fix:`use ${to.names().join(`, `)}, or reveal.define('${a}', …)`});let c=$a[n.mark]??[`fade`],l=o?Qa[s]??0:c.includes(a)||a===`fade`?Qa[a]:0,u=e===void 0?n.fallback:Math.max(0,r.dur??.8),d=i?Math.min(u,no):u,f=i?0:Math.max(0,r.stagger??0),p=new Float32Array(t.length);if(f>0&&t.length>1){let e;if(r.order)e=e=>r.order(e,n.s);else if(r.from&&n.centreOf){let t=r.from;e=e=>{let r=n.centreOf(e);return r?(r[0]-t[0])**2+(r[1]-t[1])**2:1/0}}else e=()=>0;let i=[];for(let e=0;e<t.length;e++)(!n.shownOnly||n.shownOnly(e))&&i.push(e);let a=i.map(n=>[n,Number(e(t[n]))||0]).sort((e,t)=>e[1]-t[1]||e[0]-t[0]),o=a.length>1?Math.min(f,1/(a.length-1)):0;a.forEach(([e],t)=>{p[e]=(r.reverse?a.length-1-t:t)*o})}let m=0;for(let e of p)e>m&&(m=e);return{mode:l,name:a,dur:d,total:Math.max(1e-4,d+m),spec:o,delays:p,easeFn:e=>Ne(r.ease??`smooth`,e)}}function io(e,t,n){if(e.dur<=0)return+(n>0);let r=(n*e.total-(e.delays[t]??0))/e.dur;return r<=0?0:r>=1?1:e.easeFn(r)}const ao=`
uniform highp sampler2D geo;
uniform highp sampler2D sceneDepth;
/*@upperDecl*/
const int GW = 2048;
vec4 pieceAt(int n, int r, int k) {
  int t = n * r + k;
  return texelFetch(geo, ivec2(t % GW, t / GW), 0);
}
vec4 depthPlace(vec2 fc, float d, float bias) {
  if (d >= 0.9999999) {
    return vec4(0.0);
  }
  vec2 t = vec2(fc.x / frame.fbsize.x * 2.0 - 1.0, fc.y / frame.fbsize.y * 2.0 - 1.0);
  vec2 ndc = (t - frame.region.zw) / frame.region.xy;
  // (the ground drew itself a little farther in the depth order: frame's groundClip, undone here)
  vec4 w = frame.inv * vec4(ndc, d - bias, 1.0);
  if (!(abs(w.w) > 0.000000000000000000000000000001)) {
    return vec4(0.0);
  }
  vec3 p = w.xyz / w.w;
  if (!(abs(p.x) + abs(p.y) + abs(p.z) < 1000000000000000000000000000000.0)) {
    return vec4(0.0);
  }
  return vec4(p, 1.0);
}
vec4 groundAt(vec2 fc) {
  return depthPlace(fc, texelFetch(sceneDepth, ivec2(fc), 0).r, GROUND_DEPTH_BIAS);
}
/*@upperFn*/
vec4 edgePlane(vec2 a, vec2 b, vec3 A, vec3 UA, vec3 B, vec3 UB) {
  vec3 P0 = A;
  vec3 U0 = UA;
  vec3 Q0 = B;
  if (b.x < a.x || (b.x == a.x && b.y < a.y)) {
    P0 = B;
    U0 = UB;
    Q0 = A;
  }
  vec3 n = cross(Q0 - P0, U0 - P0);
  return vec4(n, -dot(n, P0));
}
bool inSide(float s, float k) {
  return s * k > 0.0 || (s == 0.0 && k > 0.0);
}
float prismAt(float level, vec2 g) {
  /*@upperPrism*/
  if (u.ground.z > 0.5) {
    return u.ground.x * u.origin.z + u.origin.w;
  }
  // w = 1: the pieces' own ranges are being measured again (the ground changed): the layer's, never too short; a
  // piece's own never wider than the layer's (a range read where only coarse tiles were in spans a whole region)
  vec2 r = clamp(g, vec2(u.ground.x), vec2(u.ground.y));
  if (u.ground.w > 0.5) {
    r = u.ground.xy;
  }
  return mix(r.x, r.y, level) * u.origin.z + u.origin.w;
}
`,oo={decl:`uniform highp sampler2D groundDepth;`,fn:`bool upperKeeps(vec2 fc, vec4 g) {
  float kept = texelFetch(groundDepth, ivec2(fc), 0).r;
  float here = texelFetch(sceneDepth, ivec2(fc), 0).r;
  if (!(here < kept)) {
    return false;
  }
  if (u.slots2.z > 0.0) {
    vec4 under = depthPlace(fc, kept, GROUND_DEPTH_BIAS);
    vec4 top = depthPlace(fc, here, 0.0);
    if (under.w < 0.5 || top.y - under.y > u.slots2.z) {
      return false;
    }
  }
  return true;
}`,prism:`vec2 ru = clamp(g, vec2(u.ground.x), vec2(u.ground.y));
  if (u.ground.w > 0.5 || u.ground.z > 0.5) {
    ru = u.ground.xy;
  }
  return mix(ru.x - 5.0, max(ru.x, ru.y) + u.slots2.w, level) * u.origin.z + u.origin.w;`,fs:`if (!upperKeeps(gl_FragCoord.xy, g)) {
    discard;
  }`},so=`${ua}${ao}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,co={vertex:`${so}
flat out vec4 v_col;
flat out float v_idx;
flat out vec4 v_state;
flat out float v_value;
flat out vec4 v_pa;
flat out vec4 v_pb;
flat out vec4 v_pc;
flat out vec3 v_ks;
flat out vec4 v_rab;
flat out vec2 v_rc;
flat out vec4 v_box;
out vec3 v_world;
vec2 metresToRel(float e, float n) {
  return vec2(e, n) / (RE * max(frame.misc.y, 0.01));
}
int prismCorner(int k) {
  const int t[24] = int[](3, 4, 5, 0, 2, 1, 0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5);
  return t[k];
}
int footCorner(int k) {
  const int t[3] = int[](0, 2, 1);
  return t[k];
}
vec2 moved(vec2 r, vec4 box, vec4 state, vec4 motion) {
  vec2 p = r;
  if (u.reveal.x == 4.0) {
    p = box.xy + (p - box.xy) * state.z;
  }
  return box.xy + (p - box.xy) * motion.w + metresToRel(motion.x, motion.y);
}
void main() {
  bool level0 = u.ground.z > 0.5 /*@flat*/;
  int per = 24;
  if (level0) {
    per = 3;
  }
  int n = gl_VertexID / per;
  int c = 0;
  if (level0) {
    c = footCorner(gl_VertexID % 3);
  } else {
    c = prismCorner(gl_VertexID % 24);
  }
  vec4 t0 = pieceAt(n, 3, 0);
  vec4 t1 = pieceAt(n, 3, 1);
  vec4 t2 = pieceAt(n, 3, 2);
  float idx = t1.z;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  vec4 box = featAt(idx, 4);
  vec2 ra = moved(t0.xy, box, state, motion);
  vec2 rb = moved(t0.zw, box, state, motion);
  vec2 rc = moved(t1.xy, box, state, motion);
  vec3 A; vec3 DA; placeUp(ra, A, DA);
  vec3 B; vec3 DB; placeUp(rb, B, DB);
  vec3 C; vec3 DC; placeUp(rc, C, DC);
  int v = c % 3;
  vec3 P = A;
  vec3 D = DA;
  if (v == 1) {
    P = B;
    D = DB;
  } else if (v == 2) {
    P = C;
    D = DC;
  }
  vec3 corner = P + prismAt(float(c / 3), t2.xy) * D;
  gl_Position = clipOut(frame.viewProj * vec4(corner, 1.0));
  v_world = corner;
  vec3 UA = A + 1000.0 * DA;
  vec3 UB = B + 1000.0 * DB;
  vec3 UC = C + 1000.0 * DC;
  v_pa = edgePlane(rb, rc, B, UB, C, UC);
  v_pb = edgePlane(rc, ra, C, UC, A, UA);
  v_pc = edgePlane(ra, rb, A, UA, B, UB);
  v_ks = vec3(dot(v_pa.xyz, A) + v_pa.w, dot(v_pb.xyz, B) + v_pb.w, dot(v_pc.xyz, C) + v_pc.w);
  v_rab = vec4(ra, rb);
  v_rc = rc;
  v_box = box;
  v_col = colourOf(idx, state);
  v_idx = idx;
  v_state = state;
  v_value = featAt(idx, 3).z;
}
`,fragment:`${so}
flat in vec4 v_col;
flat in float v_idx;
flat in vec4 v_state;
flat in float v_value;
flat in vec4 v_pa;
flat in vec4 v_pb;
flat in vec4 v_pc;
flat in vec3 v_ks;
flat in vec4 v_rab;
flat in vec2 v_rc;
flat in vec4 v_box;
in vec3 v_world;
out vec4 outColor;
void main() {
  vec4 g = groundAt(gl_FragCoord.xy);
  if (g.w < 0.5) {
    discard;
  }
  /*@upper*/
  bool level = u.ground.z > 0.5 /*@flat*/;
  vec3 position = g.xyz;
  if (level) {
    position = v_world;
  }
  float sa = dot(v_pa.xyz, position) + v_pa.w;
  float sb = dot(v_pb.xyz, position) + v_pb.w;
  float sc = dot(v_pc.xyz, position) + v_pc.w;
  if (!level && (!inSide(sa, v_ks.x) || !inSide(sb, v_ks.y) || !inSide(sc, v_ks.z))) {
    discard;
  }
  vec2 rel = v_rab.xy * (sa / v_ks.x) + v_rab.zw * (sb / v_ks.y) + v_rc * (sc / v_ks.z);
  vec2 uv = (rel - v_box.xy) / max(v_box.zw, vec2(0.0000001));
  vec4 color = v_col;
  float alpha = color.a;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  float time = frame.eye.w;
  float value = v_value;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    float e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, uv.x));
  } else if (mode == 6.0) {
    float e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(uv)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.hover2.w, v_idx);
    return;
  }
  outColor = vec4(themed(color.rgb, max(v_state.x, v_state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`},lo=`
@group(1) @binding(1) var geo: texture_2d<f32>;
@group(2) @binding(0) var sceneDepth: texture_depth_2d;
/*@upperDecl*/
const GW = 2048;
// texel k of piece n, its record r texels long
fn pieceAt(n: i32, r: i32, k: i32) -> vec4f {
  let t = n * r + k;
  return textureLoad(geo, vec2i(t % GW, t / GW), 0);
}
// the place a depth at a pixel stands for (world metres), bias: the depth its program drew it farther by (the
// ground's: GROUND_DEPTH_BIAS); w = 0 where nothing was drawn
fn depthPlace(fc: vec2f, d: f32, bias: f32) -> vec4f {
  if (d >= 0.9999999) {
    return vec4f(0.0);
  }
  let t = vec2f(fc.x / frame.fbsize.x * 2.0 - 1.0, 1.0 - fc.y / frame.fbsize.y * 2.0);
  let ndc = (t - frame.region.zw) / frame.region.xy;
  // (the ground drew itself a little farther in the depth order: frame's groundClip, undone here)
  let w = frame.inv * vec4f(ndc, d - bias, 1.0);
  // a depth the chip could not turn back into a place (w near 0, a garbage or imprecise copy): no ground there, never a
  // NaN carried into the colour (black specks on some phone chips)
  if (!(abs(w.w) > 0.000000000000000000000000000001)) {
    return vec4f(0.0);
  }
  let p = w.xyz / w.w;
  if (!(abs(p.x) + abs(p.y) + abs(p.z) < 1000000000000000000000000000000.0)) {
    return vec4f(0.0);
  }
  return vec4f(p, 1.0);
}
// the ground under a pixel (the depth drawn so far, back to world metres); w = 0 where nothing was drawn
fn groundAt(fc: vec2f) -> vec4f {
  return depthPlace(fc, textureLoad(sceneDepth, vec2i(fc), 0), GROUND_DEPTH_BIAS);
}
/*@upperFn*/
// the vertical plane through a ground edge a–b (A, B: its ends on the ground, UA, UB: 1 km above them), the same
// numbers from either side (its ends in one order): two pieces sharing an edge split its pixels exactly, none twice,
// none missed
fn edgePlane(a: vec2f, b: vec2f, A: vec3f, UA: vec3f, B: vec3f, UB: vec3f) -> vec4f {
  var P0 = A;
  var U0 = UA;
  var Q0 = B;
  if (b.x < a.x || (b.x == a.x && b.y < a.y)) {
    P0 = B;
    U0 = UB;
    Q0 = A;
  }
  let n = cross(Q0 - P0, U0 - P0);
  return vec4f(n, -dot(n, P0));
}
// inside the side of the plane where the value k (its value at the opposite corner) lies; on the plane: one owner
fn inSide(s: f32, k: f32) -> bool {
  return s * k > 0.0 || (s == 0.0 && k > 0.0);
}
// the height of a level of the prism: 0 its foot, 1 its top; g: the lowest and highest ground under the piece (true m,
// margins included): a prism stands only as tall as the ground under its own piece (little fill on a phone's chip)
fn prismAt(level: f32, g: vec2f) -> f32 {
  /*@upperPrism*/
  if (u.ground.z > 0.5) {
    return u.ground.x * u.origin.z + u.origin.w;
  }
  // w = 1: the pieces' own ranges are being measured again (the ground changed): the layer's, never too short; a
  // piece's own never wider than the layer's (a range read where only coarse tiles were in spans a whole region)
  var r = clamp(g, vec2f(u.ground.x), vec2f(u.ground.y));
  if (u.ground.w > 0.5) {
    r = u.ground.xy;
  }
  return mix(r.x, r.y, level) * u.origin.z + u.origin.w;
}
`,uo={decl:`@group(2) @binding(1) var groundDepth: texture_2d<f32>;`,fn:`// what the part paints at this pixel: something drawn over the terrain (and, on the ground, low over it)
fn upperKeeps(fc: vec2f, g: vec4f) -> bool {
  let kept = textureLoad(groundDepth, vec2i(fc), 0).x;
  let here = textureLoad(sceneDepth, vec2i(fc), 0);
  if (!(here < kept)) {
    return false;
  }
  // (what was drawn over the terrain was drawn without the ground's bias: its height over the terrain without it)
  if (u.slots2.z > 0.0) {
    let under = depthPlace(fc, kept, GROUND_DEPTH_BIAS);
    let top = depthPlace(fc, here, 0.0);
    if (under.w < 0.5 || top.y - under.y > u.slots2.z) {
      return false;
    }
  }
  return true;
}`,prism:`var ru = clamp(g, vec2f(u.ground.x), vec2f(u.ground.y));
  if (u.ground.w > 0.5 || u.ground.z > 0.5) {
    ru = u.ground.xy;
  }
  return mix(ru.x - 5.0, max(ru.x, ru.y) + u.slots2.w, level) * u.origin.z + u.origin.w;`,fs:`if (!upperKeeps(i.pos.xy, g)) {
    discard;
  }`},fo=`${ma}${lo}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) @interpolate(flat) col: vec4f,
  @location(1) @interpolate(flat) idx: f32,
  @location(2) @interpolate(flat) state: vec4f,
  @location(3) @interpolate(flat) value: f32,
  @location(4) @interpolate(flat) pa: vec4f,
  @location(5) @interpolate(flat) pb: vec4f,
  @location(6) @interpolate(flat) pc: vec4f,
  @location(7) @interpolate(flat) ks: vec3f,
  @location(8) @interpolate(flat) rab: vec4f,
  @location(9) @interpolate(flat) rc: vec2f,
  @location(10) @interpolate(flat) box: vec4f,
  @location(11) world: vec3f,
}
/*@functions*/
fn metresToRel(e: f32, n: f32) -> vec2f {
  return vec2f(e, n) / (RE * max(frame.misc.y, 0.01));
}
// the prism's 24 corners (8 triangles, outward faces counter-clockwise): corner v + 3 · level
fn prismCorner(k: i32) -> i32 {
  var t = array<i32, 24>(3, 4, 5, 0, 2, 1, 0, 1, 4, 0, 4, 3, 1, 2, 5, 1, 5, 4, 2, 0, 3, 2, 3, 5);
  return t[k];
}
// flat: the prism's foot alone (its outward face looks down: drawn from above as the prism's back faces are)
fn footCorner(k: i32) -> i32 {
  var t = array<i32, 3>(0, 2, 1);
  return t[k];
}
fn moved(r: vec2f, box: vec4f, state: vec4f, motion: vec4f) -> vec2f {
  var p = r;
  if (u.reveal.x == 4.0) {
    p = box.xy + (p - box.xy) * state.z;
  }
  return box.xy + (p - box.xy) * motion.w + metresToRel(motion.x, motion.y);
}
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let level0 = u.ground.z > 0.5 /*@flat*/;
  var per = 24;
  if (level0) {
    per = 3;
  }
  let n = i32(vid) / per;
  var c = 0;
  if (level0) {
    c = footCorner(i32(vid) % 3);
  } else {
    c = prismCorner(i32(vid) % 24);
  }
  let t0 = pieceAt(n, 3, 0);
  let t1 = pieceAt(n, 3, 1);
  let t2 = pieceAt(n, 3, 2);
  let idx = t1.z;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  let box = featAt(idx, 4);
  let ra = moved(t0.xy, box, state, motion);
  let rb = moved(t0.zw, box, state, motion);
  let rc = moved(t1.xy, box, state, motion);
  // each corner placed once (the places at any height follow from it)
  let ua = placeUp(ra);
  let ub = placeUp(rb);
  let uc = placeUp(rc);
  let v = c % 3;
  var up = ua;
  if (v == 1) {
    up = ub;
  } else if (v == 2) {
    up = uc;
  }
  var o: O;
  let corner = up.p + prismAt(f32(c / 3), t2.xy) * up.d;
  o.pos = clipOut(frame.viewProj * vec4f(corner, 1.0));
  o.world = corner;
  // the planes of the edges opposite a, b, c, and each one's value at its opposite corner
  let A = ua.p;
  let B = ub.p;
  let C = uc.p;
  let UA = ua.p + 1000.0 * ua.d;
  let UB = ub.p + 1000.0 * ub.d;
  let UC = uc.p + 1000.0 * uc.d;
  o.pa = edgePlane(rb, rc, B, UB, C, UC);
  o.pb = edgePlane(rc, ra, C, UC, A, UA);
  o.pc = edgePlane(ra, rb, A, UA, B, UB);
  o.ks = vec3f(dot(o.pa.xyz, A) + o.pa.w, dot(o.pb.xyz, B) + o.pb.w, dot(o.pc.xyz, C) + o.pc.w);
  o.rab = vec4f(ra, rb);
  o.rc = rc;
  o.box = box;
  o.col = colourOf(idx, state);
  o.idx = idx;
  o.state = state;
  o.value = featAt(idx, 3).z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let g = groundAt(i.pos.xy);
  if (g.w < 0.5) {
    discard;
  }
  /*@upper*/
  // flat (level ground): the face lies on the ground itself: its own place, exact (the depth read back only says the
  // ground is there), and the rasterizer's coverage is the piece's: no edge tests (their rounding left specks between
  // pieces at regional zooms)
  let level = u.ground.z > 0.5 /*@flat*/;
  var position = g.xyz;
  if (level) {
    position = i.world;
  }
  let sa = dot(i.pa.xyz, position) + i.pa.w;
  let sb = dot(i.pb.xyz, position) + i.pb.w;
  let sc = dot(i.pc.xyz, position) + i.pc.w;
  if (!level && (!inSide(sa, i.ks.x) || !inSide(sb, i.ks.y) || !inSide(sc, i.ks.z))) {
    discard;
  }
  let rel = i.rab.xy * (sa / i.ks.x) + i.rab.zw * (sb / i.ks.y) + i.rc * (sc / i.ks.z);
  let uv = (rel - i.box.xy) / max(i.box.zw, vec2f(0.0000001));
  var color = i.col;
  var alpha = color.a;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  let time = frame.eye.w;
  let value = i.value;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    let e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, uv.x));
  } else if (mode == 6.0) {
    let e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(uv)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.hover2.w, i.idx);
  }
  return vec4f(themed(color.rgb, max(i.state.x, i.state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`,po=`${ua}${ao}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,mo={vertex:`${po}
flat out vec4 v_col;
flat out vec3 v_a0;
flat out vec4 v_dir;
flat out vec3 v_side;
flat out vec4 v_ends;
flat out float v_halfw;
flat out float v_len;
flat out float v_idx;
flat out vec4 v_state;
flat out float v_value;
out vec3 v_world;
int boxCorner(int k) {
  const int t[36] = int[](0, 2, 1, 1, 2, 3, 4, 5, 6, 5, 7, 6, 0, 1, 5, 0, 5, 4, 2, 6, 7, 2, 7, 3, 0, 4, 6, 0, 6, 2, 1, 3, 7, 1, 7, 5);
  return t[k];
}
int footCorner(int k) {
  const int t[6] = int[](0, 2, 1, 1, 2, 3);
  return t[k];
}
void main() {
  bool level0 = u.ground.z > 0.5 /*@flat*/;
  int per = 36;
  if (level0) {
    per = 6;
  }
  int n = gl_VertexID / per;
  int c = 0;
  if (level0) {
    c = footCorner(gl_VertexID % 6);
  } else {
    c = boxCorner(gl_VertexID % 36);
  }
  vec4 a = pieceAt(n, 5, 0);
  vec4 b = pieceAt(n, 5, 1);
  vec4 before = pieceAt(n, 5, 2);
  vec4 after = pieceAt(n, 5, 3);
  vec4 x = pieceAt(n, 5, 4);
  float idx = x.x;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  float w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  vec3 SP; vec3 SD; placeUp((c % 2) == 1 ? b.xy : a.xy, SP, SD);
  w = sizePx(w, u.size, SP);
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  float halfw = w * 0.5 + 1.0;
  float lo = prismAt(0.0, x.zw);
  float hi = prismAt(1.0, x.zw);
  vec3 PA; vec3 DA; placeUp(a.xy, PA, DA);
  vec3 PB; vec3 DB; placeUp(b.xy, PB, DB);
  vec3 A0 = PA + lo * DA;
  vec3 B0 = PB + lo * DB;
  vec3 up = normalize(DA);
  vec3 ab = B0 - A0;
  float L = max(length(ab), 0.001);
  vec3 d = ab / L;
  vec3 side = normalize(cross(up, d));
  float end = float(c % 2);
  float s = float((c / 2) % 2) * 2.0 - 1.0;
  float level = float(c / 4);
  vec3 AP = end > 0.5 ? PB : PA;
  vec3 AD = end > 0.5 ? DB : DA;
  float ext = halfw * max(mppAt(AP + lo * AD), mppAt(AP + hi * AD)) * 1.5 + 0.5;
  vec3 corner = AP + prismAt(level, x.zw) * AD + side * s * ext + d * (end * 2.0 - 1.0) * ext;
  gl_Position = clipOut(frame.viewProj * vec4(corner, 1.0));
  v_world = corner;
  vec4 col = vec4(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  v_col = col;
  v_a0 = A0;
  v_dir = vec4(d, L);
  v_side = side;
  v_ends = vec4(a.w, b.w, before.w, after.w);
  v_halfw = halfw;
  v_len = x.y;
  v_idx = idx;
  v_state = state;
  v_value = featAt(idx, 3).z;
}
`,fragment:`${po}
flat in vec4 v_col;
flat in vec3 v_a0;
flat in vec4 v_dir;
flat in vec3 v_side;
flat in vec4 v_ends;
flat in float v_halfw;
flat in float v_len;
flat in float v_idx;
flat in vec4 v_state;
flat in float v_value;
in vec3 v_world;
out vec4 outColor;
void main() {
  vec4 g = groundAt(gl_FragCoord.xy);
  if (g.w < 0.5) {
    discard;
  }
  /*@upper*/
  vec3 position = g.xyz;
  if (u.ground.z > 0.5 /*@flat*/) {
    position = v_world;
  }
  vec3 rp = position - v_a0;
  float t = dot(rp, v_dir.xyz) / v_dir.w;
  float s = dot(rp, v_side);
  float mpp = mppAt(position);
  float across = acrossAt(t, s, v_dir.w, mpp, v_ends);
  if (abs(across) > v_halfw) {
    discard;
  }
  vec4 color = v_col;
  float alpha = color.a * clamp(v_halfw - abs(across), 0.0, 1.0) * clamp((v_halfw - 1.0) * 2.0, 0.0, 1.0);
  float along = mix(v_ends.x, v_ends.y, clamp(t, 0.0, 1.0));
  float alongPx = along * v_len / max(mpp / max(frame.size.z, 0.001), 0.000001);
  float time = frame.eye.w;
  float value = v_value;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  if (!dashOn(alongPx + featAt(v_idx, 3).w - time * u.b.y, featAt(v_idx, 6))) {
    discard;
  }
  if (mode == 1.0 || mode == 3.0) {
    float p = reveal;
    if (mode == 3.0) {
      p = clamp(reveal * 2.0, 0.0, 1.0);
    }
    float soft = max(u.reveal.z, 0.001);
    alpha = alpha * (1.0 - smoothstep(p - soft, p, along)) * step(0.0001, p);
    float head = u.reveal.w * exp(-abs(along - p) / soft * 2.0) * step(along, p + soft) * (1.0 - step(1.0, p));
    color = vec4(color.rgb + vec3(head), color.a);
  } else if (mode == 2.0) {
    float k = fract(along - time * 0.35);
    color = vec4(color.rgb + vec3(u.b.x * exp(-k * 12.0)), color.a);
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  // a trail (c: now, window, on, head; along = time): drawn up to now, fading over the window, a glowing head
  if (u.b.w < 0.5 && u.c.z > 0.5) {
    float age = u.c.x - along;
    alpha = alpha * step(0.0, age) * (1.0 - smoothstep(u.c.y * 0.35, u.c.y, age));
    color = vec4(color.rgb + vec3(u.c.w * exp(-age / max(u.c.y * 0.06, 0.000001))), color.a);
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.hover2.w, v_idx);
    return;
  }
  outColor = vec4(themed(color.rgb, max(v_state.x, v_state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`},ho=`${ma}${lo}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) @interpolate(flat) col: vec4f,
  @location(1) @interpolate(flat) a0: vec3f,
  @location(2) @interpolate(flat) dir: vec4f,
  @location(3) @interpolate(flat) side: vec3f,
  @location(4) @interpolate(flat) ends: vec4f,
  @location(5) @interpolate(flat) halfw: f32,
  @location(6) @interpolate(flat) len: f32,
  @location(7) @interpolate(flat) idx: f32,
  @location(8) @interpolate(flat) state: vec4f,
  @location(9) @interpolate(flat) value: f32,
  @location(10) world: vec3f,
}
/*@functions*/
// a box's 36 corners (12 triangles, outward faces counter-clockwise): end + 2 · side + 4 · level
fn boxCorner(k: i32) -> i32 {
  var t = array<i32, 36>(0, 2, 1, 1, 2, 3, 4, 5, 6, 5, 7, 6, 0, 1, 5, 0, 5, 4, 2, 6, 7, 2, 7, 3, 0, 4, 6, 0, 6, 2, 1, 3, 7, 1, 7, 5);
  return t[k];
}
// flat: the box's foot alone, a ribbon on the ground
fn footCorner(k: i32) -> i32 {
  var t = array<i32, 6>(0, 2, 1, 1, 2, 3);
  return t[k];
}
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let level0 = u.ground.z > 0.5 /*@flat*/;
  var per = 36;
  if (level0) {
    per = 6;
  }
  let n = i32(vid) / per;
  var c = 0;
  if (level0) {
    c = footCorner(i32(vid) % 6);
  } else {
    c = boxCorner(i32(vid) % 36);
  }
  let a = pieceAt(n, 5, 0);
  let b = pieceAt(n, 5, 1);
  let before = pieceAt(n, 5, 2);
  let after = pieceAt(n, 5, 3);
  let x = pieceAt(n, 5, 4);
  let idx = x.x;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  var w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  w = sizePx(w, u.size, placeUp(select(a.xy, b.xy, (c % 2) == 1)).p);
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  let halfw = w * 0.5 + 1.0;
  let lo = prismAt(0.0, x.zw);
  let hi = prismAt(1.0, x.zw);
  let ua = placeUp(a.xy);
  let ub = placeUp(b.xy);
  let A0 = ua.p + lo * ua.d;
  let B0 = ub.p + lo * ub.d;
  let up = normalize(ua.d);
  let ab = B0 - A0;
  let L = max(length(ab), 0.001);
  let d = ab / L;
  let side = normalize(cross(up, d));
  let end = f32(c % 2);
  let s = f32((c / 2) % 2) * 2.0 - 1.0;
  let level = f32(c / 4);
  var at = ua;
  if (end > 0.5) {
    at = ub;
  }
  let ext = halfw * max(mppAt(at.p + lo * at.d), mppAt(at.p + hi * at.d)) * 1.5 + 0.5;
  let corner = at.p + prismAt(level, x.zw) * at.d + side * s * ext + d * (end * 2.0 - 1.0) * ext;
  var o: O;
  o.pos = clipOut(frame.viewProj * vec4f(corner, 1.0));
  o.world = corner;
  var col = vec4f(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  o.col = col;
  o.a0 = A0;
  o.dir = vec4f(d, L);
  o.side = side;
  o.ends = vec4f(a.w, b.w, before.w, after.w);
  o.halfw = halfw;
  o.len = x.y;
  o.idx = idx;
  o.state = state;
  o.value = featAt(idx, 3).z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let g = groundAt(i.pos.xy);
  if (g.w < 0.5) {
    discard;
  }
  /*@upper*/
  // level ground: the ribbon lies on the ground itself: its own place, exact (the depth read back only says the ground
  // is there)
  var position = g.xyz;
  if (u.ground.z > 0.5 /*@flat*/) {
    position = i.world;
  }
  let rp = position - i.a0;
  let t = dot(rp, i.dir.xyz) / i.dir.w;
  let s = dot(rp, i.side);
  let mpp = mppAt(position);
  // its caps at the line's ends, its joints (acrossAt)
  let across = acrossAt(t, s, i.dir.w, mpp, i.ends);
  if (abs(across) > i.halfw) {
    discard;
  }
  var color = i.col;
  var alpha = color.a * clamp(i.halfw - abs(across), 0.0, 1.0) * clamp((i.halfw - 1.0) * 2.0, 0.0, 1.0);
  let along = mix(i.ends.x, i.ends.y, clamp(t, 0.0, 1.0));
  let alongPx = along * i.len / max(mpp / max(frame.size.z, 0.001), 0.000001);
  let time = frame.eye.w;
  let value = i.value;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  // dashes (the feature's own pattern and offset), flowing along at u.b.y px a second (0: still)
  if (!dashOn(alongPx + featAt(i.idx, 3).w - time * u.b.y, featAt(i.idx, 6))) {
    discard;
  }
  if (mode == 1.0 || mode == 3.0) {
    var p = reveal;
    if (mode == 3.0) {
      p = clamp(reveal * 2.0, 0.0, 1.0);
    }
    let soft = max(u.reveal.z, 0.001);
    alpha = alpha * (1.0 - smoothstep(p - soft, p, along)) * step(0.0001, p);
    let head = u.reveal.w * exp(-abs(along - p) / soft * 2.0) * step(along, p + soft) * (1.0 - step(1.0, p));
    color = vec4f(color.rgb + vec3f(head), color.a);
  } else if (mode == 2.0) {
    let k = fract(along - time * 0.35);
    color = vec4f(color.rgb + vec3f(u.b.x * exp(-k * 12.0)), color.a);
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  // a trail (c: now, window, on, head; along = time): drawn up to now, fading over the window, a glowing head
  if (u.b.w < 0.5 && u.c.z > 0.5) {
    let age = u.c.x - along;
    alpha = alpha * step(0.0, age) * (1.0 - smoothstep(u.c.y * 0.35, u.c.y, age));
    color = vec4f(color.rgb + vec3f(u.c.w * exp(-age / max(u.c.y * 0.06, 0.000001))), color.a);
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.hover2.w, i.idx);
  }
  return vec4f(themed(color.rgb, max(i.state.x, i.state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`,go=`
uniform highp sampler2D geo;
uniform highp sampler2D ids;
layout(std140) uniform T { vec4 bounds; vec4 k; vec4 px; } tile;
const int GW = 2048;
vec4 pieceAt(int n, int r, int k) {
  int t = n * r + k;
  return texelFetch(geo, ivec2(t % GW, t / GW), 0);
}
int pieceOf(int i) {
  return int(texelFetch(ids, ivec2(i % GW, i / GW), 0).x + 0.5);
}
vec2 tileTexel(vec2 rel) {
  float dl = u.origin.x + rel.x - tile.bounds.x;
  float dm = tile.bounds.z - (u.origin.y + rel.y);
  return vec2(dl / tile.bounds.y, dm / tile.bounds.w) * tile.k.x;
}
vec2 tileRel(vec2 t) {
  vec2 q = t / tile.k.x;
  return vec2(tile.bounds.x + q.x * tile.bounds.y - u.origin.x, tile.bounds.z - q.y * tile.bounds.w - u.origin.y);
}
vec4 tileClip(vec2 t) {
  vec2 q = t / tile.k.x;
  return vec4(q.x * 2.0 - 1.0, tile.k.y * (1.0 - q.y * 2.0), 0.0, 1.0);
}
float tilePx(vec2 t) {
  vec2 q = clamp(t / tile.k.x, vec2(0.0), vec2(1.0));
  return max(mix(mix(tile.px.x, tile.px.y, q.x), mix(tile.px.z, tile.px.w, q.x), q.y), 0.000001);
}
float tileSizePx(float v, vec4 s, vec2 t) {
  float px = v;
  if (s.x > 0.5) {
    px = v * tilePx(t) / max(tile.k.w * frame.size.z, 0.000000001);
  }
  return clamp(px, s.y, s.z);
}
`,_o=`${ua}${go}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,vo={vertex:`${_o}
flat out vec4 v_col;
flat out float v_idx;
flat out vec4 v_state;
flat out float v_value;
out vec2 v_rel;
flat out vec4 v_box;
vec2 metresToRel(float e, float n) {
  return vec2(e, n) / (RE * max(frame.misc.y, 0.01));
}
vec2 moved(vec2 r, vec4 box, vec4 state, vec4 motion) {
  vec2 p = r;
  if (u.reveal.x == 4.0) {
    p = box.xy + (p - box.xy) * state.z;
  }
  return box.xy + (p - box.xy) * motion.w + metresToRel(motion.x, motion.y);
}
void main() {
  int n = pieceOf(gl_VertexID / 3);
  int c = gl_VertexID % 3;
  vec4 t0 = pieceAt(n, 3, 0);
  vec4 t1 = pieceAt(n, 3, 1);
  float idx = t1.z;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  vec4 box = featAt(idx, 4);
  vec2 r = moved(t0.xy, box, state, motion);
  if (c == 1) {
    r = moved(t0.zw, box, state, motion);
  } else if (c == 2) {
    r = moved(t1.xy, box, state, motion);
  }
  gl_Position = tileClip(tileTexel(r));
  v_rel = r;
  v_box = box;
  v_col = colourOf(idx, state);
  v_idx = idx;
  v_state = state;
  v_value = featAt(idx, 3).z;
}
`,fragment:`${_o}
flat in vec4 v_col;
flat in float v_idx;
flat in vec4 v_state;
flat in float v_value;
in vec2 v_rel;
flat in vec4 v_box;
out vec4 outColor;
void main() {
  vec3 position = placeAt(v_rel, 0.0);
  vec2 uv = (v_rel - v_box.xy) / max(v_box.zw, vec2(0.0000001));
  vec4 color = v_col;
  float alpha = color.a;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  float time = frame.eye.w;
  float value = v_value;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    float e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, uv.x));
  } else if (mode == 6.0) {
    float e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(uv)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(color.rgb, max(v_state.x, v_state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`},yo=`
@group(1) @binding(1) var geo: texture_2d<f32>;
@group(1) @binding(2) var ids: texture_2d<f32>;
struct T { bounds: vec4f, k: vec4f, px: vec4f }
@group(0) @binding(3) var<uniform> tile: T;
const GW = 2048;
// texel k of piece n, its record r texels long
fn pieceAt(n: i32, r: i32, k: i32) -> vec4f {
  let t = n * r + k;
  return textureLoad(geo, vec2i(t % GW, t / GW), 0);
}
// the i-th piece under this tile
fn pieceOf(i: i32) -> i32 {
  return i32(textureLoad(ids, vec2i(i % GW, i / GW), 0).x + 0.5);
}
// a place (from the layer's origin) → texels in the tile, x east and y south of its north-west corner
fn tileTexel(rel: vec2f) -> vec2f {
  let dl = u.origin.x + rel.x - tile.bounds.x;
  let dm = tile.bounds.z - (u.origin.y + rel.y);
  return vec2f(dl / tile.bounds.y, dm / tile.bounds.w) * tile.k.x;
}
// texels in the tile → a place (from the layer's origin)
fn tileRel(t: vec2f) -> vec2f {
  let q = t / tile.k.x;
  return vec2f(tile.bounds.x + q.x * tile.bounds.y - u.origin.x, tile.bounds.z - q.y * tile.bounds.w - u.origin.y);
}
// texels → clip space (the north row is the texture's first on either graphics)
fn tileClip(t: vec2f) -> vec4f {
  let q = t / tile.k.x;
  return vec4f(q.x * 2.0 - 1.0, tile.k.y * (1.0 - q.y * 2.0), 0.0, 1.0);
}
// texels per device pixel at texel t: the corners' own, between them bilinear (the quadtree keeps a tile small against
// its distance, so the distance varies smoothly over it: within a few % of each pixel's own)
fn tilePx(t: vec2f) -> f32 {
  let q = clamp(t / tile.k.x, vec2f(0.0), vec2f(1.0));
  return max(mix(mix(tile.px.x, tile.px.y, q.x), mix(tile.px.z, tile.px.w, q.x), q.y), 0.000001);
}
// a size in CSS px at texel t (core › render › drawing › size): metres by the tile's metres a texel and texels a pixel there
fn tileSizePx(v: f32, s: vec4f, t: vec2f) -> f32 {
  var px = v;
  if (s.x > 0.5) {
    px = v * tilePx(t) / max(tile.k.w * frame.size.z, 0.000000001);
  }
  return clamp(px, s.y, s.z);
}
`,bo=`${ma}${yo}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) @interpolate(flat) col: vec4f,
  @location(1) @interpolate(flat) idx: f32,
  @location(2) @interpolate(flat) state: vec4f,
  @location(3) @interpolate(flat) value: f32,
  @location(4) rel: vec2f,
  @location(5) @interpolate(flat) box: vec4f,
}
/*@functions*/
fn metresToRel(e: f32, n: f32) -> vec2f {
  return vec2f(e, n) / (RE * max(frame.misc.y, 0.01));
}
fn moved(r: vec2f, box: vec4f, state: vec4f, motion: vec4f) -> vec2f {
  var p = r;
  if (u.reveal.x == 4.0) {
    p = box.xy + (p - box.xy) * state.z;
  }
  return box.xy + (p - box.xy) * motion.w + metresToRel(motion.x, motion.y);
}
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let n = pieceOf(i32(vid) / 3);
  let c = i32(vid) % 3;
  let t0 = pieceAt(n, 3, 0);
  let t1 = pieceAt(n, 3, 1);
  let idx = t1.z;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  let box = featAt(idx, 4);
  var r = moved(t0.xy, box, state, motion);
  if (c == 1) {
    r = moved(t0.zw, box, state, motion);
  } else if (c == 2) {
    r = moved(t1.xy, box, state, motion);
  }
  var o: O;
  o.pos = tileClip(tileTexel(r));
  o.rel = r;
  o.box = box;
  o.col = colourOf(idx, state);
  o.idx = idx;
  o.state = state;
  o.value = featAt(idx, 3).z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let position = placeAt(i.rel, 0.0);
  let uv = (i.rel - i.box.xy) / max(i.box.zw, vec2f(0.0000001));
  var color = i.col;
  var alpha = color.a;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  let time = frame.eye.w;
  let value = i.value;
  if (mode == 3.0) {
    alpha = alpha * clamp(reveal * 2.0 - 1.0, 0.0, 1.0);
  } else if (mode == 4.0) {
    alpha = alpha * clamp(reveal * 3.0, 0.0, 1.0);
  } else if (mode == 5.0) {
    let e = -1.15 + 2.3 * reveal;
    alpha = alpha * (1.0 - smoothstep(e - 0.12, e + 0.12, uv.x));
  } else if (mode == 6.0) {
    let e = reveal * 1.6;
    alpha = alpha * (1.0 - smoothstep(e - 0.15, e + 0.05, length(uv)));
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(color.rgb, max(i.state.x, i.state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`,xo=`${ua}${go}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,So={vertex:`${xo}
flat out vec4 v_col;
flat out vec2 v_a0;
flat out vec3 v_dir;
flat out vec2 v_side;
flat out vec4 v_ends;
flat out float v_halfw;
flat out float v_len;
flat out float v_idx;
flat out vec4 v_state;
flat out float v_value;
out vec2 v_p;
int quadCorner(int k) {
  const int t[6] = int[](0, 2, 1, 1, 2, 3);
  return t[k];
}
void main() {
  int n = pieceOf(gl_VertexID / 6);
  int c = quadCorner(gl_VertexID % 6);
  vec4 a = pieceAt(n, 5, 0);
  vec4 b = pieceAt(n, 5, 1);
  vec4 before = pieceAt(n, 5, 2);
  vec4 after = pieceAt(n, 5, 3);
  vec4 x = pieceAt(n, 5, 4);
  float idx = x.x;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  float w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  w = tileSizePx(w, u.size, (tileTexel(a.xy) + tileTexel(b.xy)) * 0.5);
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  float halfw = w * 0.5 + 1.0;
  vec2 A = tileTexel(a.xy);
  vec2 B = tileTexel(b.xy);
  vec2 ab = B - A;
  float L = max(length(ab), 0.0001);
  vec2 d = ab / L;
  vec2 side = vec2(-d.y, d.x);
  float end = float(c % 2);
  float s = float((c / 2) % 2) * 2.0 - 1.0;
  // (wide enough wherever in the tile the segment lies: the most texels a pixel anywhere in it)
  float ext = halfw * tile.k.z + 1.0;
  vec2 at = A;
  if (end > 0.5) {
    at = B;
  }
  vec2 corner = at + side * s * ext + d * (end * 2.0 - 1.0) * ext;
  gl_Position = tileClip(corner);
  v_p = corner;
  vec4 col = vec4(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  v_col = col;
  v_a0 = A;
  v_dir = vec3(d, L);
  v_side = side;
  v_ends = vec4(a.w, b.w, before.w, after.w);
  v_halfw = halfw;
  v_len = x.y;
  v_idx = idx;
  v_state = state;
  v_value = featAt(idx, 3).z;
}
`,fragment:`${xo}
flat in vec4 v_col;
flat in vec2 v_a0;
flat in vec3 v_dir;
flat in vec2 v_side;
flat in vec4 v_ends;
flat in float v_halfw;
flat in float v_len;
flat in float v_idx;
flat in vec4 v_state;
flat in float v_value;
in vec2 v_p;
out vec4 outColor;
void main() {
  vec3 position = placeAt(tileRel(v_p), 0.0);
  vec2 rp = v_p - v_a0;
  float t = dot(rp, v_dir.xy) / v_dir.z;
  float s = dot(rp, v_side);
  float k = tilePx(v_p);
  float across = acrossAt(t, s, v_dir.z, k, v_ends);
  if (abs(across) > v_halfw) {
    discard;
  }
  vec4 color = v_col;
  float ramp = max(1.0 - 1.0 / k, min(1.0, 1.0 / k));
  float alpha = color.a * clamp((v_halfw - 0.5 + ramp * 0.5 - abs(across)) / ramp, 0.0, 1.0) * clamp((v_halfw - 1.0) * 2.0, 0.0, 1.0);
  float along = mix(v_ends.x, v_ends.y, clamp(t, 0.0, 1.0));
  float mpp = tile.k.w * k;
  float alongPx = along * v_len / max(mpp / max(frame.size.z, 0.001), 0.000001);
  float time = frame.eye.w;
  float value = v_value;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  if (!dashOn(alongPx + featAt(v_idx, 3).w - time * u.b.y, featAt(v_idx, 6))) {
    discard;
  }
  if (mode == 1.0 || mode == 3.0) {
    float p = reveal;
    if (mode == 3.0) {
      p = clamp(reveal * 2.0, 0.0, 1.0);
    }
    float soft = max(u.reveal.z, 0.001);
    alpha = alpha * (1.0 - smoothstep(p - soft, p, along)) * step(0.0001, p);
    float head = u.reveal.w * exp(-abs(along - p) / soft * 2.0) * step(along, p + soft) * (1.0 - step(1.0, p));
    color = vec4(color.rgb + vec3(head), color.a);
  } else if (mode == 2.0) {
    float q = fract(along - time * 0.35);
    color = vec4(color.rgb + vec3(u.b.x * exp(-q * 12.0)), color.a);
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  // a trail (c: now, window, on, head; along = time): drawn up to now, fading over the window, a glowing head
  if (u.b.w < 0.5 && u.c.z > 0.5) {
    float age = u.c.x - along;
    alpha = alpha * step(0.0, age) * (1.0 - smoothstep(u.c.y * 0.35, u.c.y, age));
    color = vec4(color.rgb + vec3(u.c.w * exp(-age / max(u.c.y * 0.06, 0.000001))), color.a);
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(color.rgb, max(v_state.x, v_state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`},Co=`${ma}${yo}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) @interpolate(flat) col: vec4f,
  @location(1) @interpolate(flat) a0: vec2f,
  @location(2) @interpolate(flat) dir: vec3f,
  @location(3) @interpolate(flat) side: vec2f,
  @location(4) @interpolate(flat) ends: vec4f,
  @location(5) @interpolate(flat) halfw: f32,
  @location(6) @interpolate(flat) len: f32,
  @location(7) @interpolate(flat) idx: f32,
  @location(8) @interpolate(flat) state: vec4f,
  @location(9) @interpolate(flat) value: f32,
  @location(10) p: vec2f,
}
/*@functions*/
// the quad's 6 corners: end + 2 · side
fn quadCorner(k: i32) -> i32 {
  var t = array<i32, 6>(0, 2, 1, 1, 2, 3);
  return t[k];
}
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let n = pieceOf(i32(vid) / 6);
  let c = quadCorner(i32(vid) % 6);
  let a = pieceAt(n, 5, 0);
  let b = pieceAt(n, 5, 1);
  let before = pieceAt(n, 5, 2);
  let after = pieceAt(n, 5, 3);
  let x = pieceAt(n, 5, 4);
  let idx = x.x;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  var w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  w = tileSizePx(w, u.size, (tileTexel(a.xy) + tileTexel(b.xy)) * 0.5);
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  let halfw = w * 0.5 + 1.0;
  let A = tileTexel(a.xy);
  let B = tileTexel(b.xy);
  let ab = B - A;
  let L = max(length(ab), 0.0001);
  let d = ab / L;
  let side = vec2f(-d.y, d.x);
  let end = f32(c % 2);
  let s = f32((c / 2) % 2) * 2.0 - 1.0;
  // (wide enough wherever in the tile the segment lies: the most texels a pixel anywhere in it)
  let ext = halfw * tile.k.z + 1.0;
  var at = A;
  if (end > 0.5) {
    at = B;
  }
  let corner = at + side * s * ext + d * (end * 2.0 - 1.0) * ext;
  var o: O;
  o.pos = tileClip(corner);
  o.p = corner;
  var col = vec4f(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  o.col = col;
  o.a0 = A;
  o.dir = vec3f(d, L);
  o.side = side;
  o.ends = vec4f(a.w, b.w, before.w, after.w);
  o.halfw = halfw;
  o.len = x.y;
  o.idx = idx;
  o.state = state;
  o.value = featAt(idx, 3).z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let position = placeAt(tileRel(i.p), 0.0);
  let rp = i.p - i.a0;
  let t = dot(rp, i.dir.xy) / i.dir.z;
  let s = dot(rp, i.side);
  let k = tilePx(i.p);
  // its caps at the line's ends, its joints (acrossAt)
  let across = acrossAt(t, s, i.dir.z, k, i.ends);
  if (abs(across) > i.halfw) {
    discard;
  }
  var color = i.col;
  // the edge: a device pixel's ramp as on the prisms, less the texel the ground's sampling of the overlay softens it by
  // (never under a texel: finer aliases), centred where the prisms' is (the same amount of line)
  let ramp = max(1.0 - 1.0 / k, min(1.0, 1.0 / k));
  var alpha = color.a * clamp((i.halfw - 0.5 + ramp * 0.5 - abs(across)) / ramp, 0.0, 1.0) * clamp((i.halfw - 1.0) * 2.0, 0.0, 1.0);
  let along = mix(i.ends.x, i.ends.y, clamp(t, 0.0, 1.0));
  let mpp = tile.k.w * k;
  let alongPx = along * i.len / max(mpp / max(frame.size.z, 0.001), 0.000001);
  let time = frame.eye.w;
  let value = i.value;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  // dashes (the feature's own pattern and offset), flowing along at u.b.y px a second (0: still)
  if (!dashOn(alongPx + featAt(i.idx, 3).w - time * u.b.y, featAt(i.idx, 6))) {
    discard;
  }
  if (mode == 1.0 || mode == 3.0) {
    var p = reveal;
    if (mode == 3.0) {
      p = clamp(reveal * 2.0, 0.0, 1.0);
    }
    let soft = max(u.reveal.z, 0.001);
    alpha = alpha * (1.0 - smoothstep(p - soft, p, along)) * step(0.0001, p);
    let head = u.reveal.w * exp(-abs(along - p) / soft * 2.0) * step(along, p + soft) * (1.0 - step(1.0, p));
    color = vec4f(color.rgb + vec3f(head), color.a);
  } else if (mode == 2.0) {
    let q = fract(along - time * 0.35);
    color = vec4f(color.rgb + vec3f(u.b.x * exp(-q * 12.0)), color.a);
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * reveal;
  }
  // a trail (c: now, window, on, head; along = time): drawn up to now, fading over the window, a glowing head
  if (u.b.w < 0.5 && u.c.z > 0.5) {
    let age = u.c.x - along;
    alpha = alpha * step(0.0, age) * (1.0 - smoothstep(u.c.y * 0.35, u.c.y, age));
    color = vec4f(color.rgb + vec3f(u.c.w * exp(-age / max(u.c.y * 0.06, 0.000001))), color.a);
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(color.rgb, max(i.state.x, i.state.y)) * mix(1.0, alpha, u.hover2.z), alpha);
}
`,wo={area:`over`,line:`ground`,raster:`over`,point:`above`,label:`above`,arc:`above`},To=1.5,Eo=.35;function Do(e,t){return e===!1?`free`:e===void 0||e===!0?wo[t]:e}function Oo(e,t){let n=(e,n,r)=>{if(!t)return e;for(let t of[`upperDecl`,`upperFn`,`upperPrism`,`flat`])if(r&&!e.includes(`/*@${t}*/`))throw Error(`drape: the upper variant's marker /*@${t}*/ is missing`);return e.split(`/*@upperDecl*/`).join(n.decl).split(`/*@upperFn*/`).join(n.fn).split(`/*@upperPrism*/`).join(n.prism).split(`/*@flat*/`).join(` && false`).split(`/*@upper*/`).join(n.fs)};return{wgsl:n(e.wgsl,uo,!0),glsl:{vertex:n(e.glsl.vertex,oo,!0),fragment:n(e.glsl.fragment,oo,!0)}}}function ko(e,t,n){if(!n)return e;let r=(e,t)=>{if(!e.includes(`/*@functions*/`))throw Error(`an image variant needs the /*@functions*/ marker`);return e.split(`/*@functions*/`).join(`${t}/*@functions*/`).split(`/*@color*/`).join(`color = imageAt(uv);
  alpha = alpha * color.a;
  /*@color*/`)};return{...e,wgsl:r(e.wgsl,ga(t)),glsl:{vertex:r(e.glsl.vertex,pa),fragment:r(e.glsl.fragment,pa)}}}const Ao={name:`img`,kind:`2d`,sample:`float`},jo=2048,Mo=[{name:`feat`,kind:`2d`,sample:`unfilterable`},{name:`geo`,kind:`2d`,sample:`unfilterable`}],No=e=>({write:!1,test:e?`always`:`greater-equal`}),Po=(e,t,n,r,i=`alpha`,a=!1,o=!1,s=!1)=>{let c=Oo(Ji(ko({wgsl:fo,glsl:co},Mo.length,s),{color:Hi.area.color,alpha:Hi.area.alpha,reveal:Hi.area.reveal},(({position:e,...t})=>t)(n),r,`layer '${t}'`),o);return new rs(e,{label:`area ${t} (draped${a?`, flat`:``}${o?`, over the solids`:``})`,wgsl:c.wgsl,glsl:c.glsl,blocks:[`Frame`,`L`,`Params`],textures:s?[...Mo,Ao]:Mo,sceneDepth:!0,groundDepth:o,cull:`front`,blend:i??`alpha`,depth:No(a&&!o)})},Fo=(e,t,n,r,i=`alpha`,a=!1,o=!1)=>{let s=Oo(Ji({wgsl:ho,glsl:mo},Hi.line,n,r,`layer '${t}'`),o);return new rs(e,{label:`line ${t} (draped${a?`, flat`:``}${o?`, over the solids`:``})`,wgsl:s.wgsl,glsl:s.glsl,blocks:[`Frame`,`L`,`Params`],textures:Mo,sceneDepth:!0,groundDepth:o,cull:`front`,blend:i??`alpha`,depth:No(a&&!o)})},Io=[...Mo,{name:`ids`,kind:`2d`,sample:`unfilterable`}],Lo=(e,t,n=!1)=>({label:e,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`L`,`Params`,`T`],textures:n?[...Io,Ao]:Io,cull:`none`,blend:`alpha`,depth:{write:!1,test:`always`},overlay:!0}),Ro=(e,t,n,r,i=!1)=>{let a=Ji(ko({wgsl:bo,glsl:vo},Io.length,i),{color:Hi.area.color,alpha:Hi.area.alpha,reveal:Hi.area.reveal},(({position:e,...t})=>t)(n),r,`layer '${t}'`);return e.program(Lo(`area ${t} (tiles)`,a,i))},zo=(e,t,n,r)=>{let i=Ji({wgsl:Co,glsl:So},Hi.line,n,r,`layer '${t}'`);return e.program(Lo(`line ${t} (tiles)`,i))};var Bo=class{tex=null;pool=new Float32Array(jo*4);used=0;of=null;boxes=/* @__PURE__ */ new Float64Array;lists=/* @__PURE__ */ new Map;G=1;ext=[0,0,1,1];cellAt=/* @__PURE__ */ new Int32Array(2);cellIds=/* @__PURE__ */ new Int32Array;stamp=/* @__PURE__ */ new Uint32Array;mark=0;prepare(e,t,n){if(this.of===e&&this.boxes.length===t*4)return;let r=new Float64Array(t*4),i=/* @__PURE__ */ new Float64Array(4),a=1/0,o=1/0,s=-1/0,c=-1/0;for(let e=0;e<t;e++)n(e,i),r[4*e]=i[0],r[4*e+1]=i[1],r[4*e+2]=i[2],r[4*e+3]=i[3],i[0]<a&&(a=i[0]),i[1]<o&&(o=i[1]),i[2]>s&&(s=i[2]),i[3]>c&&(c=i[3]);this.of=e,this.boxes=r,this.lists.clear(),this.used=0;let l=this.G=Math.max(1,Math.min(256,Math.round(Math.sqrt(t/4))));this.ext=[a,o,Math.max(s-a,1e-9),Math.max(c-o,1e-9)];let u=new Int32Array(l*l+1),d=(e,t)=>{let n=this.cx(r[4*e]),i=this.cx(r[4*e+2]),a=this.cy(r[4*e+1]),o=this.cy(r[4*e+3]);for(let e=a;e<=o;e++)for(let r=n;r<=i;r++)t(e*l+r)};for(let e=0;e<t;e++)d(e,e=>{u[e+1]++});for(let e=0;e<l*l;e++)u[e+1]+=u[e];let f=new Int32Array(u[l*l]),p=u.slice(0,l*l);for(let e=0;e<t;e++)d(e,t=>{f[p[t]++]=e});this.cellAt=u,this.cellIds=f,this.stamp=new Uint32Array(t),this.mark=0}cx(e){return Math.max(0,Math.min(this.G-1,Math.floor((e-this.ext[0])/this.ext[2]*this.G)))}cy(e){return Math.max(0,Math.min(this.G-1,Math.floor((e-this.ext[1])/this.ext[3]*this.G)))}select(e,t,n,r=0){let i=this.lists.get(t);if(i)return i;let a=(n[2]-n[0])*r,o=(n[3]-n[1])*r,s=n[0]-a,c=n[1]-o,l=n[2]+a,u=n[3]+o,d=this.boxes,f=this.cellIds,p=this.cellAt,m=this.stamp,h=this.G;++this.mark===4294967295&&(m.fill(0),this.mark=1);let g=this.mark;this.used>1<<22&&(this.lists.clear(),this.used=0);let _=this.used;if(l>=this.ext[0]&&s<=this.ext[0]+this.ext[2]&&u>=this.ext[1]&&c<=this.ext[1]+this.ext[3]){let e=this.cx(s),t=this.cx(l),n=this.cy(c),r=this.cy(u);for(let i=n;i<=r;i++)for(let n=e;n<=t;n++){let e=i*h+n;for(let t=p[e];t<p[e+1];t++){let e=f[t];if(m[e]===g)continue;m[e]=g;let n=4*e;if(!(d[n]>l||d[n+2]<s||d[n+1]>u||d[n+3]<c)){if(this.used===this.pool.length){let e=new Float32Array(this.pool.length*2);e.set(this.pool),this.pool=e}this.pool[this.used++]=e}}}}let v={first:_,n:this.used-_};if(this.lists.set(t,v),v.n){let t=Math.floor(_/jo),n=Math.ceil(this.used/jo),r=Math.ceil(this.used/jo);if(!this.tex||this.tex.desc.height<r){if(this.tex?.destroy(),this.tex=e.texture({label:`tile pieces`,width:jo,height:Math.max(r,Math.ceil((this.tex?.desc.height??0)*1.5),4),format:`r32f`,filter:`nearest`}),this.pool.length<this.tex.desc.height*2048){let e=new Float32Array(this.tex.desc.height*jo);e.set(this.pool),this.pool=e}this.tex.write(this.pool.subarray(0,r*jo),{width:jo,height:r})}else{if(this.pool.length<n*2048){let e=new Float32Array(n*jo);e.set(this.pool),this.pool=e}this.tex.write(this.pool.subarray(t*jo,n*jo),{y:t,width:jo,height:n-t})}}return v}get texture(){return this.tex}destroy(){this.tex?.destroy(),this.tex=null,this.lists.clear(),this.used=0,this.of=null}};function Vo(e,t,n,r){let i=Math.max(1,n.length/4),a=Math.max(1,Math.ceil(i/jo)),o=t;(!o||o.desc.height<a)&&(o?.destroy(),o=e.texture({label:r,width:jo,height:Math.max(a,Math.ceil((o?.desc.height??0)*1.25)),format:`rgba32f`,filter:`nearest`}));let s=n;return n.length!==a*2048*4&&(s=new Float32Array(a*jo*4),s.set(n)),o.write(s,{width:jo,height:a}),o}var Ho=class{i=-1;phase=0;again=!1;b=/* @__PURE__ */ new Float64Array(4);boxes=/* @__PURE__ */ new Float64Array;ext=[0,0,1,1];G=1;cells=/* @__PURE__ */ new Float32Array;want=!1;ended=-1e9;timer=0;get busy(){return this.i>=0||this.want}get running(){return this.i>=0||this.want&&performance.now()-this.ended>1e3}wake;constructor(e=null){this.wake=e}later(){this.timer||!this.want||performance.now()-this.ended>1e3||(this.timer=setTimeout(()=>{this.timer=0,this.wake?.()},1005-(performance.now()-this.ended)),this.timer.unref?.())}start(){this.i>=0?this.again=!0:(this.want=!0,this.later())}step(e,t,n,r,i,a,o,s=1){if(this.i<0&&this.want&&performance.now()-this.ended>1e3&&(this.want=!1,this.i=0,this.phase=0),this.i<0)return!1;let c=performance.now(),l=this.b;if(this.phase===0){this.boxes.length<t*4&&(this.boxes=new Float64Array(t*4)),this.i===0&&(this.ext=[1/0,1/0,-1/0,-1/0]);let e=this.boxes,r=this.ext;for(;this.i<t;){n(this.i,l);let t=4*this.i;if(e[t]=l[0],e[t+1]=l[1],e[t+2]=l[2],e[t+3]=l[3],l[0]<r[0]&&(r[0]=l[0]),l[1]<r[1]&&(r[1]=l[1]),l[2]>r[2]&&(r[2]=l[2]),l[3]>r[3]&&(r[3]=l[3]),this.i++,!(this.i&1023)&&performance.now()-c>s)return!1}this.G=Math.max(1,Math.min(64,Math.round(Math.sqrt(t/6)))),this.cells=new Float32Array(this.G*this.G*2),this.phase=1,this.i=0}let u=this.G,d=this.ext,f=Math.max(1e-9,(d[2]-d[0])/u),p=Math.max(1e-9,(d[3]-d[1])/u);if(this.phase===1){for(;this.i<u*u;){let t=this.i%u,n=this.i/u|0,r=e?.(d[0]+t*f,d[1]+n*p,d[0]+(t+1)*f,d[1]+(n+1)*p);if(this.cells[2*this.i]=r?r[0]:NaN,this.cells[2*this.i+1]=r?r[1]:NaN,this.i++,!(this.i&15)&&performance.now()-c>s)return!1}this.phase=2,this.i=0}let m=this.boxes,h=this.cells,g=e=>Math.max(0,Math.min(u-1,Math.floor((e-d[0])/f))),_=e=>Math.max(0,Math.min(u-1,Math.floor((e-d[1])/p)));for(;this.i<t;){let e=4*this.i,t=g(m[e]),n=g(m[e+2]),l=_(m[e+1]),d=_(m[e+3]),f=1/0,p=-1/0;for(let e=l;e<=d;e++)for(let r=t;r<=n;r++){let t=2*(e*u+r),n=h[t],i=h[t+1];n===n&&(n<f&&(f=n),i>p&&(p=i))}let v=this.i*i+a;if(f===1/0?(r[v]=o[0],r[v+1]=o[1]):(r[v]=f-20,r[v+1]=p+Math.max(60,.15*(p-f))),this.i++,!(this.i&1023)&&performance.now()-c>s)return!1}return this.i=-1,this.ended=performance.now(),this.again&&(this.again=!1,this.want=!0,this.later()),!0}};function Uo(e,t,n,r,i){for(let a=0;a<t;a++)e[a*n+r]=i[0],e[a*n+r+1]=i[1]}const Wo=e=>(t,n)=>e.box(t,n),Go=e=>e&&typeof e==`object`&&`props`in e?e.props??null:null;function Ko(e,t){if(e===t)return!0;let n=Go(e),r=Go(t);if(!n||!r)return!1;let i=Object.keys(n),a=Object.keys(r);if(i.length!==a.length)return!1;for(let e of i){let t=n[e],i=r[e];if(t!==i&&!(typeof t==`number`&&typeof i==`number`&&Number.isNaN(t)&&Number.isNaN(i)))return!1}return!0}var qo=class{slots=/* @__PURE__ */ new Map;items=/* @__PURE__ */ new Map;free=[];freeing=[];size=0;last={new:0,changed:0,gone:0,stayed:0};changedKeys=/* @__PURE__ */ new Set;gone=[];shapes=!1;slotOf(e){return this.slots.get(e)}next(e){let t=Array(e.length),n=/* @__PURE__ */ new Set,r={new:0,changed:0,gone:0,stayed:0};this.changedKeys=/* @__PURE__ */ new Set,this.gone=[],this.shapes=!1;for(let i=0;i<e.length;i++){let a=e[i],o=this.items.get(a.key);n.add(a.key);let s=this.slots.get(a.key);s===void 0?(s=this.free.length?this.free.pop():this.size++,this.slots.set(a.key,s),r.new++,this.changedKeys.add(a.key),this.shapes=!0):o&&o.d===a.d||o&&o.geometry===a.geometry&&Ko(o.d,a.d)?r.stayed++:(r.changed++,this.changedKeys.add(a.key),(!o||o.geometry!==a.geometry)&&(this.shapes=!0)),this.items.set(a.key,a),t[i]=s}for(let[e,t]of this.slots)n.has(e)||(this.slots.delete(e),this.items.delete(e),this.freeing.push(t),this.gone.push(t),r.gone++,this.shapes=!0);return this.last=r,t}built(){this.freeing.length&&(this.free.push(...this.freeing),this.freeing=[])}clear(){this.slots.clear(),this.items.clear(),this.free=[],this.freeing=[],this.size=0,this.changedKeys.clear(),this.gone=[]}};const Jo=Object.freeze([0,0,1e6,0]),Yo=e=>!!e&&typeof e==`object`&&!Array.isArray(e)&&(`px`in e||`m`in e);function Xo(e){return Yo(e)?`m`in e?e.m:e.px:e}function Zo(e,t=`size`){if(!Yo(e))return Jo;let n=e.min??0,r=e.max??1e6;if(!(n>=0)||!(r>=n))throw new W(`E-DATA-001`,`${t}: min ${e.min} and max ${e.max} are not a range of CSS px`,{where:t,fix:`${t}: { m: 40, min: 4, max: 32 }`});return[+(`m`in e),n,r,0]}function Qo(e,t=`size`){return e?Zo({m:0,min:e.min,max:e.max},t):Jo}function $o(e,t,n,r){let i=t[0]>.5?e/Math.max(n*r,1e-9):e;return Math.min(t[2],Math.max(t[1],i))}const es={color:`#ffffff`,mix:.45},ts=/* @__PURE__ */ new Set([`data`,`feature`,`filter`,`reveal`,`hover`,`selected`,`hoverGroup`,`select`,`when`,`transition`]),ns=/* @__PURE__ */ new Set([`opacity`,`order`,`above`,`below`,`when`,`transition`,`label`,`lift`,`depthBias`]);var rs=class{main;pickP=null;desc;g;ok=!1;failed=!1;constructor(e,t){this.g=e,this.desc=t,this.main=e.program(t),this.main.ready.then(()=>{this.ok=!0},()=>{this.failed=!0})}get ready(){return this.main.ready}pick(){return this.pickP??=this.g.program({...this.desc,label:`${this.desc.label} pick`,blend:`none`})}destroy(){this.main.destroy(),this.pickP?.destroy(),this.pickP=null}};function is(e,t){if(!e)return;let n=e=>{let n=t[0],r=t[1],i=t[2],a=t[3];for(let t=0;t<e.length;t++){let o=e[t],s=o[0],c=o[1];s<n&&(n=s),s>i&&(i=s),c<r&&(r=c),c>a&&(a=c)}t[0]=n,t[1]=r,t[2]=i,t[3]=a};switch(e.type){case`Point`:{let n=e.coordinates[0],r=e.coordinates[1];n<t[0]&&(t[0]=n),n>t[2]&&(t[2]=n),r<t[1]&&(t[1]=r),r>t[3]&&(t[3]=r);break}case`MultiPoint`:case`LineString`:n(e.coordinates);break;case`MultiLineString`:case`Polygon`:for(let t of e.coordinates)n(t);break;case`MultiPolygon`:for(let t of e.coordinates)for(let e of t)n(e)}}const as=(e,t)=>{if(Array.isArray(t))return t.some(t=>as(e,t));if(t&&typeof t==`object`){let n=t,r=Number(e);return(n.min===void 0||r>=n.min)&&(n.max===void 0||r<=n.max)}return e===t||e!=null&&String(e)===String(t)};function os(e,t,n){if(!e)return!0;if(typeof e==`function`)return!!e(t,n);let r=t.props??t;for(let[t,n]of Object.entries(e))if(!as(r?.[t],n))return!1;return!0}var ss=class e extends la{entries=[];byKey=/* @__PURE__ */ new Map;bySlot=[];join=new qo;get joined(){return this.join.last}origin=new Ga(0,0);drawnOrigin=this.origin;g=null;feat=null;featData=/* @__PURE__ */ new Float32Array;rows=0;L=/* @__PURE__ */ new Float32Array(72);lbuf=null;pbuf=null;planes=4;slots=(/* @__PURE__ */ new Int8Array(7)).fill(-1);dirtyPlanes=0;k0=/* @__PURE__ */ new Float32Array(4);k3=/* @__PURE__ */ new Float32Array(4);k6=/* @__PURE__ */ new Float32Array(4);planesWanted(){return 4}featureTexture=!0;progs=[];step=250;plan=null;src=null;seen={v:-1,sv:-1,arr:null,len:-1,scales:``};mixT=1;mixFrom=0;hoverSet=null;selKey=`\0`;sel=/* @__PURE__ */ new Set;building=!1;newest=-1e9;left=/* @__PURE__ */ new Map;static rememberLeft=5e3;groundSeen=-1;groundTime=-1e9;stateDirty=!0;stateWide=!0;born=!1;built=!1;sctx=null;motionUsed=!1;constructor(e){super(e);let t=e.data;if(!(Array.isArray(t)||t instanceof Ur))throw new W(`E-DATA-001`,`${this.constructor.name.toLowerCase()} '${this.id}': data must be an array or a source`,{where:this.id,fix:`pass data: [...] or data.geojson('…')`});this.paramBlock??=Gi({})}get source(){return this.props.data instanceof Ur?this.props.data:null}get fades(){return!0}drapeMark(){return null}legacySits(){}get sits(){let e=this.drapeMark();if(!e)return`free`;let t=this.props.drape;return t===void 0?this.legacySits()??wo[e]:Do(t,e)}get order(){let e=this.drapeMark();return this.props.order??((e===`area`||e===`line`)&&this.sits===`above`?4.5:void 0)}attribution(){let e=this.source;if(!e)return[];let t=e;return(t.shapes||t.rows?[t.shapes?.attribution,t.rows?.attribution]:[e.attribution]).filter(e=>!!e)}async attach(e){if(await super.attach(e),this.checkScales(),this.sctx={loader:e.loader,error:t=>e.world.error(t)},this.src=this.source,e.graphics&&(this.g=e.graphics,this.lbuf=e.graphics.buffer(`uniform`,288,`${this.id} layer`),this.pbuf=e.graphics.buffer(`uniform`,this.paramBlock.size,`${this.id} params`),this.progs=this.programs(this.code()),this.preparePrograms(),await Promise.all(this.progs.map(e=>e.ready)),this.props.pickable!==!1))for(let e of this.progs)e.pick()}code(){let e=this.props.reveal,t=this.attached?.world.context===`video`,n=typeof e==`string`?t?e:void 0:e?t?e.video??e.mode:e.app:void 0,r=n?to.get(String(n)):void 0;return{...this.props.shader??{},...r?.wgsl?{reveal:r.wgsl}:{}}}async setShader(e){let t=this.props;if(this.props={...this.props,shader:e},!this.g)return;let n=[];try{n=this.programs(this.code()),await Promise.all(n.map(e=>e.ready))}catch(e){throw n.forEach(e=>e.destroy()),this.props=t,e}this.progs.forEach(e=>e.destroy()),this.progs=n,this.preparePrograms()}setProps(e){let t=e,n=`data`in t&&t.data!==this.props.data,r=this.sits,i=this.draped;return super.setProps(t),this.sits!==r&&this.g&&this.progs.length&&(this.progs.forEach(e=>e.destroy()),this.progs=this.programs(this.code()),this.preparePrograms(),this.draped!==i&&(this.freeGeometry(),this.built=!1,this.building=!1,this.seen.sv=-2,this.seen.v=-2,this.seen.arr=null)),n&&(this.src=this.source,this.seen.v=-2),Object.keys(t).some(e=>!ns.has(e))&&(this.seen.scales=`\0`),Object.keys(t).some(e=>ts.has(e))&&(this.stateDirty=!0,this.stateWide=!0),`reveal`in t&&(this.plan=null),this}detach(){this.progs.forEach(e=>e.destroy()),this.progs=[];for(let e of[this.feat,this.lbuf,this.pbuf])e?.destroy();this.feat=null,this.lbuf=this.pbuf=null,this.g=null,this.freeGeometry(),this.built=!1,this.building=!1,this.seen={v:-1,sv:-1,arr:null,len:-1,scales:``},this.join.clear(),this.entries=[],this.bySlot=[],this.byKey=/* @__PURE__ */ new Map,this.scalesRead=[],this.plan=null,this.featData=/* @__PURE__ */ new Float32Array,this.rows=0,this.layoutKey=-1,this.born=!1,this.stateDirty=!0,this.stateWide=!0,super.detach()}letGo(){for(let[e,t]of this.byKey)this.left.set(e,t.born);this.freeGeometry(),this.built=!1,this.building=!1,this.seen={v:-1,sv:-1,arr:null,len:-1,scales:``},this.join.clear(),this.entries=[],this.bySlot=[],this.byKey=/* @__PURE__ */ new Map,this.feat?.destroy(),this.feat=null,this.featData=/* @__PURE__ */ new Float32Array,this.rows=0,this.layoutKey=-1}read(t){let n=this.byKey,r=this.src,i=this.born?t:-1e9,a=this.left,o=-1e9,s=[];if(r){let e=r.features();for(let t=0;t<e.length;t++){let n=e[t];s.push({id:n.id,key:String(n.id),d:n,geometry:n.geometry,cuts:n.cuts})}}else{let e=this.props.data;for(let t=0;t<e.length;t++){let n=e[t],r=n,i=r.geometry===void 0?Array.isArray(r.at)?{type:`Point`,coordinates:[r.at[0],r.at[1]]}:null:r.geometry,a=r.id??t;s.push({id:a,key:String(a),d:n,geometry:i??null})}}let c=this.join.next(s),l=Array(s.length);for(let e=0;e<s.length;e++){let t=s[e],r=n.get(t.key),u=r?r.born:a.get(t.key)??i;u>o&&(o=u),l[e]={id:t.id,key:t.key,d:t.d,geometry:t.geometry,cuts:t.cuts,index:c[e],born:u}}if(this.newest=o,n.size){let e=new Set(l.map(e=>e.key));for(let[t,r]of n)e.has(t)||(a.delete(t),a.set(t,r.born))}for(let e of l)a.delete(e.key);if(a.size>e.rememberLeft){let t=a.size-e.rememberLeft;for(let e of a.keys()){if(t--<=0)break;a.delete(e)}}let u=Array(this.join.size);for(let e of l)u[e.index]=e;return this.bySlot=u,l}st={color:[1,1,1,1],number:0,value:0};colourInto(e){let t=this.st.color;if(typeof e==`string`||e==null){let n=Kr(e);t[0]=n[0],t[1]=n[1],t[2]=n[2],t[3]=n[3]}else t[0]=e[0]??1,t[1]=e[1]??1,t[2]=e[2]??1,t[3]=e[3]??1;return t}valuesChanged(){}uniforms(e){}sizeUnits(){return Jo}get draped(){let e=this.sits;return this.drapeable()&&(e===`ground`||e===`over`)&&(this.props.on??`ground`)===`ground`&&!this.props.shader?.position}upperNow(e){return this.draped&&!!e.world.standing}bbox=null;drapeable(){return!1}rangesStale(){return!1}measuring(){return!1}range=[0,0];groundEmpty=!1;sampled=[0,0];flatNow=!1;groundRange(e,t){let n=1/0,r=-1/0,i=180,a=90,o=-180,s=-90,c=t.length/2,l=Math.max(1,Math.floor(c/3e3));for(let u=0;u<c;u+=l){let c=t[2*u],l=t[2*u+1],d=e.world.heightAt(c,l);c<i&&(i=c),c>o&&(o=c),l<a&&(a=l),l>s&&(s=l),Number.isNaN(d)||(d<n&&(n=d),d>r&&(r=d))}for(let t=0;t<=24;t++)for(let c=0;c<=24;c++){let l=e.world.heightAt(i+(o-i)*c/24,a+(s-a)*t/24);Number.isNaN(l)||(l<n&&(n=l),l>r&&(r=l))}this.groundEmpty=!Number.isFinite(n),Number.isFinite(n)||(n=0,r=0),this.sampled=[n,r],this.range=[Math.min(0,n)-20,r+Math.max(150,.3*(r-Math.min(0,n)))]}groundAt(e,t,n){if((this.props.on??`ground`)!==`ground`)return 0;let r=e.world.heightAt(t,n);return Number.isNaN(r)?0:r}sync(e){let t=this.src,n=e.s,r=e.now;t&&this.sctx&&(t.start(this.sctx),this.srcStarted=!0),t&&t.frame(n,this.sctx);let i=!1,a=!1;if(t)t.shapeVersion!==this.seen.sv&&(i=!0,this.seen.sv=t.shapeVersion),t.version!==this.seen.v&&(a=!0,this.seen.v=t.version);else{let e=this.props.data;(e!==this.seen.arr||e.length!==this.seen.len||this.seen.v===-2)&&(i=!0,a=!0,this.seen.arr=e,this.seen.len=e.length,this.seen.v=0)}let o=i||a;if(i||a){this.entries=this.read(r);let e=/* @__PURE__ */ new Map;for(let t of this.entries)e.set(t.key,t);this.byKey=e,this.readOrders(n)&&!i&&this.built&&this.reordered();let t=this.join.last;i=this.join.shapes||!this.built&&this.entries.length>0,a=i||t.changed>0,(i||a)&&(this.stateDirty=!0,this.stateWide=!0,this.plan=null,this.join.gone.length&&this.clearSlots(this.join.gone),i||this.valuesChanged())}let s=!this.built;i&&this.entries.length&&(this.originFor(),this.ensureTexture(),this.building=!0),this.building&&this.entries.length&&this.build(e)!==!1&&(this.building=!1,this.built=!0,this.groundSeen=-1,this.join.built(),this.drawnOrigin=this.origin),o&&(this.born=!0);let c=this.styleProps().filter(ui),l=c.length!==this.scalesRead.length||c.some((e,t)=>e!==this.scalesRead[t]);(a||i||l&&this.entries.length)&&(this.prepareScales(c),this.scalesRead=c);let u=c.map(e=>e.version).join(`,`);(a||i||u!==this.seen.scales)&&(this.seen.scales=u,this.restyle(e,!s)),this.flushPlanes()}orders=/* @__PURE__ */ new Float64Array;orderVer=0;sortedOf=null;readOrders(e){let t=this.props.feature?.order;if(!t)return!1;if(this.orders.length<this.bySlot.length){let e=new Float64Array(Math.max(64,this.bySlot.length*2)).fill(NaN);e.set(this.orders),this.orders=e}let n=!1;for(let r of this.entries){let i=Number(t(r.d,e))||0;this.orders[r.index]!==i&&(this.orders[r.index]=i,n=!0)}return n&&this.orderVer++,n}buildList(){if(!this.props.feature?.order)return this.entries;let e=this.sortedOf;if(e&&e.of===this.entries&&e.ver===this.orderVer)return e.list;let t=this.orders,n=this.entries.map((e,t)=>t);n.sort((e,n)=>t[this.entries[e].index]-t[this.entries[n].index]||e-n);let r=n.map(e=>this.entries[e]);return this.sortedOf={of:this.entries,ver:this.orderVer,list:r},r}reordered(){this.building=!0}clearSlots(e){let t=this.featData;for(let n of e)for(let e of[0,1,2,3]){let r=this.at(n,e);r+3<t.length&&(t[r]=0,t[r+1]=0,t[r+2]=0,t[r+3]=0)}this.touch(0),this.touch(1),this.touch(2),this.touch(3),this.lookGen++}prepareScales(e){if(!e.length)return;let t=this.src,n=t?`${t.id}:${t.version}`:`${this.id}:${++this.readGen}`,r=this.entries.map(e=>e.d);for(let t of e)t.prepare(r,n)}readGen=0;srcStarted=!1;scalesRead=[];checkScales(){let e=this.source;if(!e?.byView)return;let t=this.styleProps().filter(ui).find(e=>e.auto);if(t)throw new W(`E-DATA-003`,`scale.${t.kind} over '${e.id}' (a source that follows the view) has no domain: its classes would change as the view moves`,{where:this.id,fix:t.kind===`category`?`give its categories: scale.category(d => d.props.kind, { domain: ['a', 'b', 'c'] })`:`give its domain: scale.${t.kind}(d => d.props.value, { domain: [10, 20, 40, 80] }) (or a threshold scale with your breaks)`})}originFor(){let e=null,t=[1/0,1/0,-1/0,-1/0];for(let e of this.entries)is(e.geometry,t);if(Number.isFinite(t[0])&&(e=t),this.bbox=e&&Number.isFinite(e[0])?[e[0],e[1],e[2],e[3]]:null,!e||!Number.isFinite(e[0])){this.origin=new Ga(0,0),this.step=1e3;return}this.origin=new Ga((e[0]+e[2])/2,(e[1]+e[3])/2);let n=this.origin.rel(e[0],e[1]),r=this.origin.rel(e[2],e[3]);this.step=this.props.detail?.step??Za(Ka(this.origin,n[0],n[1],r[0],r[1]))}ensureTexture(){let e=Math.max(1,Math.ceil(this.bySlot.length/512));if(e!==this.rows){let t=this.featData,n=this.rows;if(this.rows=e,this.featData=new Float32Array(512*e*7*4),n)for(let r=0;r<7;r++)this.featData.set(t.subarray(r*n*512*4,Math.min((r+1)*n*512*4,r*n*512*4+e*512*4)),r*e*512*4);this.lookGen++,this.stateDirty=!0,this.stateWide=!0,this.layoutKey=-1}this.layout(this.planes)}layoutKey=-1;layout(e){if(e|=this.planesWanted(),e===this.layoutKey)return;this.planes=e,this.layoutKey=e;let t=0;for(let n=0;n<7;n++)this.slots[n]=e&1<<n?t++:-1;this.g&&this.featureTexture&&this.rows&&(this.feat?.destroy(),this.feat=this.g.texture({label:`${this.id} features`,width:512,height:this.rows*t,format:`rgba32f`,filter:`nearest`})),this.dirtyPlanes=e,this.lookGen++}touch(e){this.dirtyPlanes|=1<<e}flushPlanes(){let e=this.dirtyPlanes&this.planes;if(this.dirtyPlanes=0,!e||!this.feat)return;let t=512*this.rows*4,n=this.featData;for(let r=0;r<7;r++)e&1<<r&&this.feat.write(n.subarray(r*t,(r+1)*t),{y:this.slots[r]*this.rows,height:this.rows})}restyleSeconds(){return this.props.restyle??this.props.transition??0}isShown(e){let t=this.at(e,2);return this.featData.length>t+3&&this.featData[t+3]>0&&this.featData[t+2]>0}at(e,t){return((t*this.rows+Math.floor(e/512))*512+e%512)*4}restyle(e,t){if(!this.entries.length)return;this.ensureTexture();let n=this.featData,r=e.s,i=this.k0,a=this.k3,o=this.k6,s=this.restyleSeconds(),c=t&&s>0&&!this.onTiles,l=this.mixT,u=!1,d=!1,f=!1,p=!0;for(let t of this.entries){let s=t.index,m=this.at(s,0),h=this.at(s,1),g=this.at(s,3),_=this.at(s,6);for(let e=0;e<4;e++)n[h+e]=n[h+e]+(n[m+e]-n[h+e])*l;n[g+1]=n[g+1]+(n[g]-n[g+1])*l;let v=this.style(t,r),y=v.color,b=v.extra;if(n[m]=y[0],n[m+1]=y[1],n[m+2]=y[2],n[m+3]=y[3],n[g]=v.number,n[g+2]=v.value,v.base!==void 0&&(n[g+3]=v.base),b&&(n[_]=b[0],n[_+1]=b[1],n[_+2]=b[2],n[_+3]=b[3]),(!c||t.born===e.now)&&(n[h]=y[0],n[h+1]=y[1],n[h+2]=y[2],n[h+3]=y[3],n[g+1]=v.number),p){p=!1;for(let e=0;e<4;e++)i[e]=n[m+e],a[e]=n[g+e],o[e]=b?n[_+e]:0}else!u&&(n[m]!==i[0]||n[m+1]!==i[1]||n[m+2]!==i[2]||n[m+3]!==i[3])&&(u=!0),!d&&(n[g]!==a[0]||n[g+1]!==a[1]||n[g+2]!==a[2]||n[g+3]!==a[3])&&(d=!0),b&&!f&&(n[_]!==o[0]||n[_+1]!==o[1]||n[_+2]!==o[2]||n[_+3]!==o[3])&&(f=!0)}this.mixT=+!c,this.mixFrom=e.now;let m=this.planes&-76;this.layout(m|+!!u|(s>0?2:0)|(d?8:0)|(f?64:0)),this.touch(0),this.touch(1),this.touch(3),this.touch(6),this.lookGen++}boxes(){if(!(this.planesWanted()&16))return;let e=this.featData,t=this.origin,n=[0,0,0,0];for(let r of this.entries){n[0]=n[1]=1/0,n[2]=n[3]=-1/0,is(r.geometry,n);let i=this.at(r.index,4);if(!Number.isFinite(n[0])){e[i]=0,e[i+1]=0,e[i+2]=1e-6,e[i+3]=1e-6;continue}let a=t.x(n[0]),o=t.y(n[1]),s=t.x(n[2]),c=t.y(n[3]);e[i]=(a+s)/2,e[i+1]=(o+c)/2,e[i+2]=Math.max(1e-9,(s-a)/2),e[i+3]=Math.max(1e-9,(c-o)/2)}this.touch(4),this.flushPlanes(),this.lookGen++}animating(e){return this.plan?.mode===2||/\btime\b/.test(Object.values(this.props.shader??{}).join(` `))}revealSeconds(){return this.plan?.total??this.props.transition??.35}frame(e){if(!this.g||!this.progs.length||!this.lbuf||(this.sync(e),this.entries.length||(this.mixT=1),!this.entries.length||!this.built||!this.feat&&this.featureTexture))return[];let t=!!(this.props.feature?.when||this.props.filter);this.plan??=ro(this.props.reveal,this.bySlot.map(e=>e?e.d:null),{mark:this.mark,context:e.world.context,s:e.s,fallback:this.props.transition??.35,centreOf:e=>{let t=Ze(e?.geometry??null);return t?[(t[0]+t[2])/2,(t[1]+t[3])/2]:null},shownOnly:n=>{let r=this.bySlot[n];if(!r)return!1;if(!t)return!0;try{return this.gateOf(r,e.s)>0}catch{return!0}}});let n=e.loadNow??e.now,r=e.camera.moving?this.groundEvery*3:this.groundEvery;e.world.groundVersion!==this.groundSeen&&(n-this.groundTime>r||this.groundEmpty)&&(this.groundSeen=e.world.groundVersion,this.groundTime=n,this.ground(e));let i=this.restyleSeconds();return this.mixT<1&&(this.mixT=i>0&&!this.onTiles?Math.min(1,(e.now-this.mixFrom)/1e3/i):1),this.updateState(e),this.flushPlanes(),this.writeUniforms(e),this.paramsDirty&&(this.pbuf.write(this.paramBlock.pack(this.paramValues)),this.paramsDirty=!1,this.lookGen++),this.frameBlock=e.frame,this.draws(e)}prepare(e){if(!this.g||!this.progs.length||!this.lbuf)return!0;if(this.sync(e),!this.entries.length)return!this.src||this.srcStarted&&this.src.status().pending.length===0;if(!this.built||this.building)return!1;if(this.warm(e),this.onTiles&&e.world.prepareGround){let t=this.frame(e).flatMap(e=>e.ground?[e.ground]:[]);return e.world.prepareGround(t,e)}return!0}preparePrograms(){}graphicsBusy(){return this.progs.some(e=>!e.ok&&!e.failed)}solidsAround(){let e=this.attached?.world;return!!e&&(e.standing===!0||e.elementsOfKind(`shape`).length>0||e.elementsOfKind(`extrusion`).length>0)}warm(e){}groundEvery=200;lastShown=-1;birthsLast=!1;gates=/* @__PURE__ */ new Float32Array;gatePaths=null;gateReads=``;gateAt=-1e9;gateOf(e,t){let n=this.props,r=1;if(n.feature?.when){let i;try{i=n.feature.when(e.d,t)}catch(e){throw new W(`E-WHEN-001`,`a feature 'when' threw: ${e.message}`,{where:`layer '${this.id}' feature`,fix:`check the fields it reads exist on the feature and on s`,cause:e})}r=ut(i,`layer '${this.id}' feature`)}return n.filter&&!os(n.filter,e.d,t)&&(r=0),r}updateState(e){let t=e.s,n=this.props,r=this.plan,i=e.now,a=t.user.selection,o=a.join(``);o!==this.selKey&&(this.selKey=o,this.sel=new Set(a.map(String)),this.stateDirty=!0);let s=r.dur*1e3,c=i-this.newest<s&&this.entries.some(e=>i-e.born<s),l=!!(n.feature?.when||n.filter),u=this.shown!==this.lastShown||c||this.birthsLast;this.birthsLast=c;let d=!1;if(l){let e=typeof performance<`u`?performance.now():Date.now();(this.gatePaths===null||this.gates.length!==this.bySlot.length||this.stateDirty||e-this.gateAt>1e3||lt(t,this.gatePaths)!==this.gateReads)&&(d=!0,this.gateAt=e)}else this.gates.length!==this.bySlot.length&&(this.gates=new Float32Array(this.bySlot.length).fill(1));if(!(this.stateDirty||u||d))return;let f=this.stateDirty;this.lastShown=this.shown,this.stateDirty=!1;let p=this.featData,m=this.hoverSet,h=r.spec?.feature,g=!1,_=f,v=[],y=this.stateWide;this.stateWide=!1;let b=f,x=null,S=t;d&&(this.gates.length!==this.bySlot.length&&(this.gates=new Float32Array(this.bySlot.length).fill(1)),x=/* @__PURE__ */ new Set,S=ct(t,x));for(let e of this.entries){let n=e.index,a=this.at(n,2),o=io(r,n,this.shown);i-e.born<s&&s>0&&(o*=r.easeFn(Math.max(0,Math.min(1,(i-e.born)/s)))),d&&(this.gates[n]=l?this.gateOf(e,S):1);let c=this.gates[n]??1,u=this.at(n,5);if(h){let n=h(e.d,o,t);c*=n.opacity??1,p[u]=n.offset?.[0]??0,p[u+1]=n.offset?.[1]??0,p[u+2]=n.offset?.[2]??0,p[u+3]=n.scale??1,g=!0}else(p[u+3]!==1||this.motionUsed)&&(p[u]=0,p[u+1]=0,p[u+2]=0,p[u+3]=1,g=!0);let f=+!!m?.has(e.key),x=+!!this.sel.has(e.key);(p[a]!==f||p[a+1]!==x||p[a+2]!==o||p[a+3]!==c)&&(p[a+2]!==o||p[a+3]!==c?y=!0:v.length<64?v.push(n):y=!0,(p[a]!==f||p[a+1]!==x||p[a+3]!==c)&&(b=!0),p[a]=f,p[a+1]=x,p[a+2]=o,p[a+3]=c,_=!0)}if(this.motionUsed=!!h,h&&!(this.planes&32)&&this.layout(this.planes|32),x&&(this.gatePaths=[...x],this.gateReads=lt(t,this.gatePaths)),!_&&!g)return;let C=this.lookGen,w=!b&&!g&&this.shown>0&&this.shown<1;if(_&&(this.touch(2),w||this.lookGen++),g&&(this.touch(5),this.lookGen++),_&&!g&&!y){let e=[];for(let t of v){let n=Ze(this.bySlot[t]?.geometry??null);n&&e.push(n[0],n[1],n[2],n[3])}this.partial={from:C,to:this.lookGen,boxes:e}}}writeUniforms(e){let t=this.L,n=this.props,r=e.camera,i=this.plan,a=this.drawnOrigin.offset(r.lon,r.lat),o=n.on??`ground`,s=n.hover===!1?null:n.hover??(n.tooltip||n.onHover||n.hoverGroup?es:null),c=n.selected===!1?null:n.selected??{color:`#ffd166`,mix:.55,outline:2},l=s?Kr(s.color??`#ffffff`):[1,1,1,1],u=c?Kr(c.color??`#ffd166`):[1,1,1,1],d=typeof n.reveal==`object`?n.reveal:{};t.set([a[0],a[1],o===`ground`?e.world.exaggeration:1,n.lift??0],0),t.set([n.opacity??1,this.shown,this.mixT,this.rows],4),t.set([i.mode,+!!this.motionUsed,d.soft??.06,d.head?.glow??0],8),t.set([l[0],l[1],l[2],s&&s.color?s.mix??.45:0],12),t.set([s?.raise??0,s?.outline??0,+(n.blend===`multiply`||n.blend===`screen`),this.pickNo],16),t.set([u[0],u[1],u[2],c&&c.color?c.mix??.55:0],20),t.set([c?.raise??0,c?.outline??0,0,0],24),t[40]=this.range[0],t[41]=this.range[1],t[42]=0,t[43]=+!!this.rangesStale();let f=this.slots,p=this.k0,m=this.k3;if(t[44]=f[0],t[45]=f[1],t[46]=f[3],t[47]=f[4],t[48]=f[2],t[49]=f[5],t[50]=0,t[51]=0,this.draped&&e.world.standing){let n=this.sits===`over`,r=this.bbox,i=n&&r?e.world.topIn?.(r[0],r[1],r[2],r[3])??null:null;t[50]=n?0:To,t[51]=n?Math.max(3.5,i===null?80:i-this.sampled[0]+5):3.5}if(t[52]=p[0],t[53]=p[1],t[54]=p[2],t[55]=p[3],t[56]=m[0],t[57]=m[1],t[58]=m[2],t[59]=m[3],t.set(this.sizeUnits(),60),t[64]=f[6],t[65]=0,t[66]=0,t[67]=0,t[68]=this.k6[0],t[69]=this.k6[1],t[70]=this.k6[2],t[71]=this.k6[3],this.flatNow=!1,this.draped){let n=e.world.exaggeration,i=(this.sampled[1]-this.sampled[0])*n<1,a=2*Math.max(1,r.groundY+r.dist*Math.sin(r.pitch)-this.range[1]*n)*Math.tan(r.fov/2)/Math.max(1,r.height);(i||(this.range[1]-this.range[0])*n/a<.33)&&(this.flatNow=!0,t[40]=t[41]=i?this.sampled[0]:(this.range[0]+this.range[1])/2,t[42]=1)}this.uniforms(e),this.lbuf.write(t)}lookGen=0;frameBlock=null;get onTiles(){let e=this.attached?.world;if(!e||!this.draped||(this.props.blend??`alpha`)!==`alpha`)return!1;let t=this.props.groundMarks,n=t&&t!==`auto`?t:e.groundMarksChosen?e.groundMarks:void 0;return n?n===`tiles`:e.groundMarks===`tiles`&&this.relief()&&!this.selfMoving()}relief(){return(this.sampled[1]-this.sampled[0])*(this.attached?.world.exaggeration??1)>=1}selfMoving(){return this.plan?.mode===2||/\btime\b/.test(Object.values(this.props.shader??{}).join(` `))}tileReady=!1;tileProgramsMade(e){this.tileReady=!1,Promise.all(e.map(e=>e.ready)).then(()=>{this.tileReady=!0,this.lookGen++,(this.attached?.world)?.invalidate?.()},e=>this.attached?.world.error?.(e))}groundDraw(e,t,n,r,i,a){let o=t[5]>0&&t[5]<1||t[6]>0&&t[6]<1,s=0;for(let e=4;e<40;e++){let n=o&&(e===5||e===6)?0:t[e];s=Math.imul(s,31)+Math.round(n*4096)|0}let c=this.selfMoving(),l=`${this.id}/${e}/${this.lookGen}/${s}${c?`/${r}`:o?`/f${Math.floor(r/80)}`:``}`,u=this.frameBlock,d=this.partial;return{version:l,since:d&&d.to===this.lookGen&&!c?{version:`${this.id}/${e}/${d.from}/${s}`,boxes:d.boxes}:void 0,ready:this.tileReady,where:this.bbox,tile:(e,t)=>a(e,u,n,t)}}partial=null;call(e,t,n,r=this.lbuf){return{program:e.main,blocks:[n,r,this.pbuf],textures:[this.feat],...t,pickProgram:this.props.pickable===!1?void 0:()=>e.pick()}}pickInfo(e){let t=this.bySlot[e];if(!t)return null;let n=t.d.props??t.d;return{element:this.owner?.id??this.id,id:t.id,props:n,d:t.d,index:e}}hovered(e){this.hoverSet=e,this.stateDirty=!0}litKeys(e=!0,t=!0){let n=[],r=e?this.hoverSet:null;if(r)for(let e of r)this.byKey.has(e)&&n.push(e);if(t)for(let e of this.sel)this.byKey.has(e)&&!r?.has(e)&&n.push(e);return n}groupOf(e){let t=this.props.hoverGroup,n=this.byKey.get(e);if(!t||!n)return/* @__PURE__ */ new Set([e]);let r=t(n.d),i=/* @__PURE__ */ new Set;for(let e of this.entries)t(e.d)===r&&i.add(e.key);return i}entryOf(e){return this.byKey.get(String(e))}status(){let e=super.status();if(!this.attached)return{...e,ready:!1,pending:[`attaching`]};let t=this.src?.status();if(t&&e.failed.push(...t.failed),this.target<=0&&this.shown<=0)return e;t&&e.pending.push(...t.pending),this.attached.graphics&&(this.target>0||this.shown>0)&&this.attached.world.groundVersion!==this.groundSeen&&this.entries.length&&this.built&&e.pending.push(`reading the ground`),this.building&&e.pending.push(`building its shapes`),this.built&&this.entries.length&&this.graphicsBusy()&&e.pending.push(`building its programs`),this.mixT<1&&e.pending.push(`restyling`),this.built&&this.entries.length&&this.measuring()&&e.pending.push(`measuring its pieces' ground ranges`);let n=(this.plan?.dur??0)*1e3,r=this.attached.world.now;return r-this.newest<n&&this.entries.some(e=>r-e.born<n)&&e.pending.push(`new features revealing`),e.ready=!e.pending.length,e}get size(){return this.src?this.src.size:this.props.data.length}readouts(){return{count:()=>this.size}}};const cs=`${Sn}
layout(std140) uniform T { vec4 misc; vec4 hov; } u;
layout(std140) uniform Params { /*@params*/ } params;
uniform highp sampler2D atlas;
/*@functions*/
`,ls={vertex:`${cs}
layout(location = 0) in vec4 at;
layout(location = 1) in vec4 box;
layout(location = 2) in vec4 uvr;
layout(location = 3) in vec4 extra;
out vec2 v_uv;
out vec2 v_t;
flat out vec4 v_extra;
vec2 quadCorner(int v) {
  float x = (v == 1 || v == 4 || v == 5) ? 1.0 : 0.0;
  float y = (v == 2 || v == 3 || v == 5) ? 1.0 : 0.0;
  return vec2(x, y);
}
void main() {
  vec2 c = quadCorner(gl_VertexID);
  vec4 clip = frame.viewProj * vec4(at.xyz, 1.0);
  vec2 px = (vec2(c.x - 0.5, 0.5 - c.y) * box.xy + vec2(box.z, -box.w)) * frame.size.z;
  gl_Position = clipOut(vec4(clip.xy + px / (frame.size.xy * 0.5) * clip.w, clip.z - u.misc.x * clip.w, clip.w));
  v_uv = mix(uvr.xy, uvr.zw, c);
  v_t = c;
  v_extra = extra;
}
`,fragment:`${cs}
in vec2 v_uv;
in vec2 v_t;
flat in vec4 v_extra;
out vec4 outColor;
void main() {
  vec4 color = textureLod(atlas, v_uv, 0.0);
  float hov = (v_extra.w == 1.0 || v_extra.w == 3.0) ? 1.0 : 0.0;
  color = vec4(mix(color.rgb, u.hov.rgb, u.hov.a * hov * color.a), color.a);
  float alpha = color.a * v_extra.x;
  vec2 uv = v_t;
  float time = frame.eye.w;
  float reveal = v_extra.y;
  if (u.misc.z == 11.0) {
    alpha = alpha * step(uv.x, reveal + 0.0001);
  } else if (u.misc.z == 13.0) {
    alpha = alpha * (1.0 - smoothstep(reveal * 1.2 - 0.2, reveal * 1.2, uv.x));
  } else if (u.misc.z != 12.0 && u.misc.z != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.004) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.misc.y, v_extra.z);
    return;
  }
  outColor = vec4(themed(color.rgb, step(0.5, v_extra.w)), alpha);
}
`},us=`${In}
struct T { misc: vec4f, hov: vec4f }
@group(0) @binding(1) var<uniform> u: T;
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
@group(1) @binding(0) var atlas: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
struct I { @builtin(vertex_index) vid: u32, @location(0) at: vec4f, @location(1) box: vec4f, @location(2) uvr: vec4f, @location(3) extra: vec4f }
struct O { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) t: vec2f, @location(2) @interpolate(flat) extra: vec4f }
/*@functions*/
fn quadCorner(v: i32) -> vec2f {
  let x = select(0.0, 1.0, v == 1 || v == 4 || v == 5);
  let y = select(0.0, 1.0, v == 2 || v == 3 || v == 5);
  return vec2f(x, y);
}
@vertex fn vs(i: I) -> O {
  let c = quadCorner(i32(i.vid));
  let clip = frame.viewProj * vec4f(i.at.xyz, 1.0);
  let px = (vec2f(c.x - 0.5, 0.5 - c.y) * i.box.xy + vec2f(i.box.z, -i.box.w)) * frame.size.z;
  var o: O;
  o.pos = clipOut(vec4f(clip.xy + px / (frame.size.xy * 0.5) * clip.w, clip.z - u.misc.x * clip.w, clip.w));
  o.uv = mix(i.uvr.xy, i.uvr.zw, c);
  o.t = c;
  o.extra = i.extra;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  var color = textureSampleLevel(atlas, smp, i.uv, 0.0);
  let hov = select(0.0, 1.0, i.extra.w == 1.0 || i.extra.w == 3.0);
  color = vec4f(mix(color.rgb, u.hov.rgb, u.hov.a * hov * color.a), color.a);
  var alpha = color.a * i.extra.x;
  let uv = i.t;
  let time = frame.eye.w;
  let reveal = i.extra.y;
  if (u.misc.z == 11.0) {
    alpha = alpha * step(uv.x, reveal + 0.0001);
  } else if (u.misc.z == 13.0) {
    alpha = alpha * (1.0 - smoothstep(reveal * 1.2 - 0.2, reveal * 1.2, uv.x));
  } else if (u.misc.z != 12.0 && u.misc.z != 14.0) {
    alpha = alpha * reveal;
  }
  /*@color*/
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.004) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.misc.y, i.extra.z);
  }
  return vec4f(themed(color.rgb, step(0.5, i.extra.w)), alpha);
}
`,ds=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e;var fs=class{size=1024;canvas;g;grown=0;map=/* @__PURE__ */ new Map;shelves=[];top=0;dirty=null;clock=0;full=!1;gen=0;constructor(){this.canvas=new OffscreenCanvas(this.size,this.size),this.g=this.canvas.getContext(`2d`,{willReadFrequently:!0})}get(e,t,n,r){let i=this.map.get(e);if(i)return i.used=this.clock,i;t=Math.min(this.size,Math.ceil(t)),n=Math.min(this.size,Math.ceil(n));let a=this.shelves.find(e=>e.h>=n&&e.h<=n*1.4&&e.x+t<=this.size);a||(this.top+n>this.size&&(this.size<4096?this.grow():(this.reset(),this.full=!0)),a={y:this.top,h:n,x:0},this.shelves.push(a),this.top+=n);let o={x:a.x,y:a.y,w:t,h:n,used:this.clock};return a.x+=t,this.g.clearRect(o.x,o.y,t,n),r(this.g,o.x,o.y),this.map.set(e,o),this.dirty=this.dirty?[Math.min(this.dirty[0],o.y),Math.max(this.dirty[1],o.y+n)]:[o.y,o.y+n],o}reset(){this.gen++,this.map.clear(),this.shelves=[],this.top=0,this.g.clearRect(0,0,this.size,this.size),this.dirty=[0,this.size]}grow(){let e=this.canvas,t=this.size*2,n=new OffscreenCanvas(t,t),r=n.getContext(`2d`,{willReadFrequently:!0});r.drawImage(e,0,0),this.canvas=n,this.g=r,this.size=t,this.grown++,this.dirty=[0,t]}},ps=class e extends ss{kind=`label`;mark=`text`;drapeMark(){return`label`}legacySits(){return this.props.occlude===void 0?void 0:this.props.occlude?`ground`:`above`}featureTexture=!1;inst=null;tbuf=null;tex=null;atlas=null;data=/* @__PURE__ */ new Float32Array;cap=0;count=0;T=/* @__PURE__ */ new Float32Array(8);places=[];static hold=.25;fade=/* @__PURE__ */ new Map;last=0;constructor(e){if(super(e),typeof e.text!=`function`)throw new W(`E-DATA-001`,`text '${this.id}' needs text: d => string`,{where:this.id,fix:`text({ data, text: d => d.props.name })`})}programs(e){let t=Ji({wgsl:us,glsl:ls},Hi.text,e,this.paramBlock,`layer '${this.id}'`);return[new rs(this.g,{label:`text ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`T`,`Params`],textures:[{name:`atlas`,kind:`2d`,sample:`float`}],instance:{stride:16,attributes:[{name:`at`,location:0,size:4,offset:0},{name:`box`,location:1,size:4,offset:4},{name:`uvr`,location:2,size:4,offset:8},{name:`extra`,location:3,size:4,offset:12}]},blend:`alpha`,depth:{write:!1,test:this.sits===`above`?`always`:`less`}})]}async attach(e){await super.attach(e),e.graphics&&(this.tbuf=e.graphics.buffer(`uniform`,32,`${this.id} text`))}styleProps(){return[this.props.color,Xo(this.props.size)]}style(e,t){return{color:Kr(ds(this.props.color,e.d,t,`#ffffff`)),number:ds(Xo(this.props.size),e.d,t,12),value:0}}steps=/* @__PURE__ */ new Map;restepped=0;shownPx(e,t,n,r,i){if(n[0]<.5)return Math.min(n[2],Math.max(n[1],t));let a=8*Math.log2(Math.max(.5,$o(t,n,r,i))),o=this.steps.get(e),s=o??Math.round(a);return o!==void 0&&Math.abs(a-o)>.7&&(s=Math.round(a),this.restepped++),this.steps.set(e,s),Math.min(n[2],Math.max(n[1],2**(s/8)))}valuesChanged(){(this.places.length||this.entries.length)&&this.build()}anchors=/* @__PURE__ */ new WeakMap;build(){this.places=[],this.colors.clear(),this.hidden.clear(),this.steps.clear();let e=this.props.at,t=this.props.priority,n=this.attached?this.attached.world.state:void 0;for(let r of this.entries){let i,a,o;if(e){let t=e(r.d);i=t[0],a=t[1],o=t[2]}else{let e=r.geometry,t=e?this.anchors.get(e):void 0;if(t===void 0){let n=Xe(e)[0]??tt(e);t=n?[n[0],n[1],n[2]]:null,e&&this.anchors.set(e,t)}if(!t)continue;i=t[0],a=t[1],o=t[2]}let s=n?this.props.text(r.d,n):this.props.text(r.d,void 0);this.places.push({lon:i,lat:a,h:o,i:r.index,key:r.key,prio:t&&Number(t(r.d))||0,text:s==null?``:String(s)})}this.places.sort((e,t)=>t.prio-e.prio||(e.key<t.key?-1:+(e.key>t.key))),this.boxes()}ground(){}pics=/* @__PURE__ */ new Map;picsNew=0;picsFrame=-1;pictureOf(e,t,n,r,i){this.picsFrame!==i&&(this.picsFrame=i,this.picsNew=0);let a=`${e}|${t}|${n.toFixed(2)}|${r}`,o=this.pics.get(a);if(o&&o.gen===this.atlas.gen)return o;if(this.picsNew>=12)return null;this.picsNew++;let s={...this.picture(e,t,n,r),gen:this.atlas.gen};return this.pics.size>4e3&&this.pics.clear(),this.pics.set(a,s),s}picture(e,t,n,r){let i=this.props,a=typeof i.halo==`number`?{width:i.halo}:i.halo??{},o=a.width??2,s=qr(Kr(a.color??`#000000b0`)),c=`${i.font?.weight??600} ${Math.round(n*r)}px ${i.font?.family??`system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`}`,l=this.atlas,u=l.g;u.font=c;let d=e.split(`
`),f=Math.round(n*r*1.25),p=Math.max(...d.map(e=>u.measureText(e).width)),m=p+2*(o*r+2),h=f*d.length+2*(o*r+2),g=l.get(`${c}|${t}|${o}|${s}|${e}`,m,h,(e,n,i)=>{e.font=c,e.textBaseline=`middle`,e.textAlign=`center`,e.lineJoin=`round`,d.forEach((a,c)=>{let l=n+m/2,u=i+o*r+2+f*(c+.5);o>0&&(e.strokeStyle=s,e.lineWidth=o*2*r,e.strokeText(a,l,u)),e.fillStyle=t,e.fillText(a,l,u)})});return{r:g,w:g.w/r,h:g.h/r,tw:p/r}}draws(t){let n=t.graphics,r=t.camera,i=t.world.exaggeration,a=this.props;if(t.s,!this.tbuf||!this.places.length)return[];this.atlas??=new fs,(!this.tex||this.texSize!==this.atlas.size)&&(this.tex?.destroy(),this.texSize=this.atlas.size,this.tex=n.texture({label:`${this.id} atlas`,width:this.atlas.size,height:this.atlas.size,format:`rgba8`,filter:`linear`}),this.atlas.dirty=[0,this.atlas.size]);let o=this.places.length,s=n.width/Math.max(1,r.width),c=this.featData,l=this.plan?.mode??0;(!this.inst||this.cap<o)&&(this.inst?.destroy(),this.cap=Math.max(64,o*2),this.inst=n.buffer(`instance`,this.cap*16*4,`${this.id} labels`),this.data=new Float32Array(this.cap*16));let u=Math.min(.25,(t.now-this.last)/1e3);this.last=t.now,this.atlas.clock++;let d=a.collide===`own`?[]:t.labelRoom??[],f=a.collide!==!1,p=a.anchor??`center`,m=a.offset??[0,0],h=0;this.placedNow.clear(),this.waiting=0,this.held=0;let g=Zo(a.size,`text '${this.id}' size`),_=r.eye,v=2*Math.tan(r.fov/2)/Math.max(1,r.height),y=this.data,b=e=>`${a.font?.weight??600} ${Math.round(e*s)}px ${a.font?.family??`system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`}`;for(let e of this.places){let n=e.i,o=this.at(n,2),x=c[o+3],S=c[o+2];if(x<=0||S<=0){this.fade.delete(e.key),this.state.delete(e.key),this.changedAt.delete(e.key);continue}if(!r.facing(e.lon,e.lat)){this.settleFade(e.key);continue}let C=e.h===void 0?(a.on??`ground`)===`ground`?t.world.heightAt(e.lon,e.lat):0:e.h,w=(Number.isNaN(C)?0:C)*i+(a.lift??0),T=de(r,e.lon,e.lat,w),E=r.projectWorld(T[0],T[1],T[2]);if(!E||E[0]<-200||E[1]<-100||E[0]>r.width+200||E[1]>r.height+100){this.settleFade(e.key);continue}if(i>0&&this.sits!==`above`&&this.occluded(t,n,T)){this.fadeTo(e.key,!1,u);continue}let D=e.text;if(D===``)continue;if(l===12&&S<1){let e=/^([^\d-]*)(-?[\d,]*\.?\d+)(.*)$/.exec(D);if(e){let t=Number(e[2].replace(/,/g,``)),n=(e[2].split(`.`)[1]??``).length;D=`${e[1]}${(t*S).toLocaleString(`en`,{minimumFractionDigits:n,maximumFractionDigits:n})}${e[3]}`}}this.colorsGen!==this.lookGen&&(this.colors.clear(),this.colorsGen=this.lookGen);let O=this.at(n,0),k=this.at(n,3),A=this.colors.get(n);if(A===void 0){let e=this.q4;for(let t=0;t<4;t++)e[t]=c[O+t];A=qr(e),this.colors.set(n,A)}let j=this.shownPx(e.key,c[k],g,Math.hypot(T[0]-_[0],T[1]-_[1],T[2]-_[2]),v),M=`${D}|${j.toFixed(2)}|${s}`,N=this.measured.get(M);if(!N){let e=this.atlas.g;e.font=b(j);let t=(typeof a.halo==`number`?a.halo:a.halo?.width??2)+2;N={w:Math.max(...D.split(`
`).map(t=>e.measureText(t).width))/s+2*t,h:j*1.25*D.split(`
`).length+2*t},this.measured.size>8e3&&this.measured.clear(),this.measured.set(M,N)}let P=N.w,F=N.h,I=m[0]+(p===`left`?P/2:p===`right`?-P/2:0),ee=m[1]+(p===`top`?F/2:p===`bottom`?-F/2:0),L=E[0]+I,R=E[1]+ee,te=(P-6)/2,ne=(F-4)/2,re=L-te,z=R-ne,B=L+te,ie=R+ne,ae=!f||!d.some(e=>re<e[2]&&B>e[0]&&z<e[3]&&ie>e[1]);ae&&d.push([re,z,B,ie]);let V=this.decide(e.key,ae,t.now),H=this.fadeTo(e.key,V,u);if(H<=0)continue;let oe=this.pictureOf(D,A,j,s,t.now);if(!oe){this.waiting++;continue}let se=this.atlas,ce=oe.r,le=se.size,ue=S;if(l===11&&S<1){let e=[...D].length,t=Math.floor(S*e);se.g.font=b(j);let n=se.g.measureText([...D].slice(0,t).join(``)).width/s;ue=(oe.w-oe.tw)/2/oe.w+n/oe.w,t===0&&(ue=0)}let U=h*16;y[U]=T[0],y[U+1]=T[1],y[U+2]=T[2],y[U+3]=0,y[U+4]=oe.w,y[U+5]=oe.h,y[U+6]=I,y[U+7]=ee,y[U+8]=ce.x/le,y[U+9]=ce.y/le,y[U+10]=(ce.x+ce.w)/le,y[U+11]=(ce.y+ce.h)/le,y[U+12]=H*x*(a.opacity??1),y[U+13]=ue,y[U+14]=n,y[U+15]=c[o]+2*c[o+1],h++}this.count=h;for(let[n,r]of this.changedAt)t.now-r>=e.hold*1e3&&this.changedAt.delete(n);if(this.atlas.dirty){let[e,t]=this.atlas.dirty,n=this.atlas.g.getImageData(0,e,this.atlas.size,t-e);this.tex.write(n.data,{y:e,height:t-e}),this.atlas.dirty=null}if(!h)return[];this.inst.write(y.subarray(0,h*16));let x=a.hover===!1?null:a.hover??(a.tooltip||a.onHover?{color:`#ffd166`}:null),S=Kr(x?.color??`#ffd166`);this.T.set([a.depthBias===void 0?4e-4:a.depthBias*1e-5,this.pickNo,l,0,S[0],S[1],S[2],x?x.mix??.6:0]),this.tbuf.write(this.T);let C=this.progs[0];return[{program:C.main,blocks:[t.frame,this.tbuf,this.pbuf],textures:[this.tex],instances:this.inst,count:6,instanceCount:h,pickProgram:a.pickable===!1?void 0:()=>C.pick()}]}settleFade(e){let t=this.fade.get(e);t===void 0||t<=0||t>=1||(this.rising.get(e)?this.fade.set(e,1):(this.fade.set(e,0),this.turns++))}rising=/* @__PURE__ */ new Map;placedNow=/* @__PURE__ */ new Set;state=/* @__PURE__ */ new Map;changedAt=/* @__PURE__ */ new Map;held=0;decide(t,n,r){let i=this.state.get(t);if(i===void 0)return this.state.set(t,n),n;if(n===i)return i;let a=this.changedAt.get(t);return a!==void 0&&r-a<e.hold*1e3?(this.held++,i):(this.state.set(t,n),this.changedAt.set(t,r),n)}waiting=0;fadeTo(e,t,n){this.rising.set(e,t),this.placedNow.add(e);let r=this.fade.get(e)??0,i=Math.max(0,Math.min(1,r+(t?1:-1)*n/.2));return r>0!=i>0&&this.turns++,this.fade.set(e,i),i}turns=0;measured=/* @__PURE__ */ new Map;colorsGen=-1;setProps(e){let t=e;return super.setProps(t),(`text`in t||`priority`in t||`at`in t)&&this.entries.length&&(this.build(),this.attached?.world&&this.attached.world.invalidate?.()),this}texSize=0;colors=/* @__PURE__ */ new Map;q4=[0,0,0,0];hidden=/* @__PURE__ */ new Map;occlusionTurn=0;occlusionFrame=-1;occluded(e,t,n){this.occlusionFrame!==e.now&&(this.occlusionFrame=e.now,this.occlusionTurn=0);let r=this.hidden.get(t);if(r===void 0&&this.occlusionTurn>=24)return!0;if(r===void 0||this.occlusionTurn<24&&(e.camera.moving||(t+Math.floor(e.now/100))%8==0)){this.occlusionTurn++;let r=this.behindGround(e,n);return this.hidden.set(t,r),r}return r}behindGround(e,t){let n=e.camera,r=n.eye,i=e.world.exaggeration;if(n.morph<.5)return!1;for(let a=1;a<12;a++){let o=a/12,s=t[0]+(r[0]-t[0])*o,c=t[1]+(r[1]-t[1])*o,l=he(n,s,c,t[2]+(r[2]-t[2])*o),u=e.world.heightAt(l[0],l[1]);if(!Number.isNaN(u)&&u*i>c+5)return!0}return!1}status(){let e=super.status();return[...this.fade.values()].some(e=>e>0&&e<1)&&(e.pending.push(`labels making room`),e.ready=!1),this.waiting>0&&(e.pending.push(`${this.waiting} labels' pictures being drawn`),e.ready=!1),this.held>0&&(e.pending.push(`${this.held} labels holding a change (0.25 s)`),e.ready=!1),e}freeGeometry(){for(let e of[this.inst,this.tbuf,this.tex])e?.destroy();this.inst=this.tbuf=null,this.tex=null,this.atlas=null,this.cap=0}readouts(){return{count:()=>this.size,shown:()=>this.count,turns:()=>this.turns,restepped:()=>this.restepped}}};const ms=e=>new ps(e),hs={vertex:`${Tn}${kn}
layout(location = 0) in float r;
layout(location = 3) in vec4 v;
layout(location = 4) in float vc;
layout(location = 5) in float card;
out vec4 v_ecol;
float copyHash(float c, float s) {
  return fract(sin(c * 12.9898 + s * 78.233) * 43758.5453);
}
void main() {
  vec4 r0 = recAt(r, 0);
  vec4 r1 = recAt(r, 1);
  vec4 r2 = recAt(r, 2);
  tileAt(r2.w);
  int code = int(v.w + 0.5);
  int e = min(code / 1024, 1);
  float c = float(code % 1024);
  vec4 E0 = si.e[e * 3];
  vec4 E1 = si.e[e * 3 + 1];
  vec4 E2 = si.e[e * 3 + 2];
  v_ecol = vec4(0.0);
  if (copyHash(c, 3.1) > si.d.x || E0.w <= 0.0) {
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
    return;
  }
  float age = fract(frame.eye.w / E0.w + c / max(E1.w, 1.0) + r2.z);
  float hx = copyHash(c, r2.z) * 2.0 - 1.0;
  float hz = copyHash(c + 17.0, r2.z) * 2.0 - 1.0;
  vec3 at = E0.xyz + vec3(hx * E1.y * age + E2.z * age, E1.x * age, hz * E1.y * age + E2.w * age);
  vec3 q0 = at + (v.xyz - E0.xyz) * mix(1.0, E1.z, age);
  float cc = cos(r1.w);
  float s = sin(r1.w);
  float g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  float keep = keepOf(max(r1.x, r1.z), placeOf(r0.x, g + si.a.x * 0.5 * r1.y, r0.y));
  vec3 q = q0 * r1.xyz * keep;
  vec3 rr = vec3(q.x * cc - q.z * s, q.y, q.x * s + q.z * cc);
  vec3 position = placeOf(r0.x + rr.x, g + rr.y, r0.y + rr.z);
  v_ecol = vec4(unpackRGB(vc), mix(E2.x, E2.y, age) * smoothstep(0.0, 0.1, age) * t.v.w);
  gl_Position = clipOut(frame.viewProj * vec4(towardEye(position), 1.0));
}
`,fragment:`${En}
in vec4 v_ecol;
out vec4 outColor;
void main() {
  if (frame.pick.x > 0.5 || frame.pick.y > 0.5 || v_ecol.a <= 0.003) {
    discard;
  }
  outColor = vec4(themed(v_ecol.rgb, 0.0), v_ecol.a);
}
`},gs=`${Rn}${Vn}
struct I { @location(0) r: f32, @location(3) v: vec4f, @location(4) vc: f32, @location(5) card: f32 }
struct OE { @builtin(position) pos: vec4f, @location(0) col: vec4f }
// a copy's own number 0..1 (its way, whether the governor keeps it)
fn copyHash(c: f32, s: f32) -> f32 {
  return fract(sin(c * 12.9898 + s * 78.233) * 43758.5453);
}
@vertex fn vs(i: I) -> OE {
  var o: OE;
  let r0 = recAt(i.r, 0);
  let r1 = recAt(i.r, 1);
  let r2 = recAt(i.r, 2);
  tileAt(r2.w);
  let code = i32(i.v.w + 0.5);
  let e = min(code / 1024, 1);
  let c = f32(code % 1024);
  let E0 = si.e[e * 3];
  let E1 = si.e[e * 3 + 1];
  let E2 = si.e[e * 3 + 2];
  if (copyHash(c, 3.1) > si.d.x || E0.w <= 0.0) {
    o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  let age = fract(frame.eye.w / E0.w + c / max(E1.w, 1.0) + r2.z);
  let hx = copyHash(c, r2.z) * 2.0 - 1.0;
  let hz = copyHash(c + 17.0, r2.z) * 2.0 - 1.0;
  let at = E0.xyz + vec3f(hx * E1.y * age + E2.z * age, E1.x * age, hz * E1.y * age + E2.w * age);
  let q0 = at + (i.v.xyz - E0.xyz) * mix(1.0, E1.z, age);
  // placed as the solids' clones: scaled, turned, on its ground, its set's range on screen kept
  let cc = cos(r1.w);
  let s = sin(r1.w);
  let g = groundAt(r0.x, r0.y) * t.v.x + r0.z;
  let keep = keepOf(max(r1.x, r1.z), placeOf(r0.x, g + si.a.x * 0.5 * r1.y, r0.y));
  let q = q0 * r1.xyz * keep;
  let rr = vec3f(q.x * cc - q.z * s, q.y, q.x * s + q.z * cc);
  let position = placeOf(r0.x + rr.x, g + rr.y, r0.y + rr.z);
  o.col = vec4f(unpackRGB(i.vc), mix(E2.x, E2.y, age) * smoothstep(0.0, 0.1, age) * t.v.w);
  o.pos = clipOut(frame.viewProj * vec4f(towardEye(position), 1.0));
  return o;
}
@fragment fn fs(o: OE) -> @location(0) vec4f {
  if (frame.pick.x > 0.5 || frame.pick.y > 0.5 || o.col.a <= 0.003) {
    discard;
  }
  return vec4f(themed(o.col.rgb, 0.0), o.col.a);
}
`,_s=2048,vs=`${Bn.replace(`/*@fieldvertex*/`,`o.col = fieldColor(o.col, fieldValue(i32(vi)));`).replace(`/*@fieldfragment*/`,`fieldNear = fieldKeep();`)}

@group(1) @binding(4) var fieldv: texture_2d<f32>;
const FW = 2048;
// texel k of the field's texture, row by row
fn fieldTexel(k: i32) -> f32 {
  return textureLoad(fieldv, vec2i(k % FW, k / FW), 0).x;
}
// vertex vi's value: its epoch's, the next one's mixed in by its share (one missing: the other)
fn fieldValue(vi: i32) -> f32 {
  let a = fieldTexel(FW + vi);
  let f = fieldTexel(5);
  if (f <= 0.0) {
    return a;
  }
  let b = fieldTexel(FW * (1 + i32(fieldTexel(6))) + vi);
  if (a < -1.0e30) {
    return b;
  }
  if (b < -1.0e30) {
    return a;
  }
  return mix(a, b, f);
}
// a value's colour over the vertex's own: the ramp between its two nearest colours, by how much; its own colour where
// there is none, or below the least drawn
fn fieldColor(own: vec3f, v: f32) -> vec3f {
  if (fieldTexel(0) < 0.5 || v < -1.0e30 || v < fieldTexel(3)) {
    return own;
  }
  let lo = fieldTexel(1);
  let hi = fieldTexel(2);
  let x = clamp((v - lo) / max(hi - lo, 0.000001), 0.0, 1.0) * 255.0;
  let i = i32(floor(x));
  let j = min(i + 1, 255);
  let c = mix(unpackRGB(fieldTexel(8 + i)), unpackRGB(fieldTexel(8 + j)), x - f32(i));
  return mix(own, c, fieldTexel(4));
}
// shown and a subject: a theme keeps its colours
fn fieldKeep() -> f32 {
  return step(0.5, fieldTexel(0)) * step(0.001, fieldTexel(4)) * fieldTexel(7);
}
`;if(vs.includes(`/*@field`))throw Error(`solid-field: the solids program lost its field hooks`);const ys=(e,t,n)=>{let r=e.lastIndexOf(`void main() {`);if(r<0||!e.includes(t))throw Error(`solid-field: the solids program lost its field hooks`);return`${e.slice(0,r)}
uniform highp sampler2D fieldv;
const int FW = 2048;
float fieldTexel(int k) {
  return texelFetch(fieldv, ivec2(k % FW, k / FW), 0).x;
}
float fieldValue(int vi) {
  float a = fieldTexel(FW + vi);
  float f = fieldTexel(5);
  if (f <= 0.0) {
    return a;
  }
  float b = fieldTexel(FW * (1 + int(fieldTexel(6))) + vi);
  if (a < -1.0e30) {
    return b;
  }
  if (b < -1.0e30) {
    return a;
  }
  return mix(a, b, f);
}
vec3 fieldColor(vec3 own, float v) {
  if (fieldTexel(0) < 0.5 || v < -1.0e30 || v < fieldTexel(3)) {
    return own;
  }
  float lo = fieldTexel(1);
  float hi = fieldTexel(2);
  float x = clamp((v - lo) / max(hi - lo, 0.000001), 0.0, 1.0) * 255.0;
  int i = int(floor(x));
  int j = min(i + 1, 255);
  vec3 c = mix(unpackRGB(fieldTexel(8 + i)), unpackRGB(fieldTexel(8 + j)), x - float(i));
  return mix(own, c, fieldTexel(4));
}
float fieldKeep() {
  return step(0.5, fieldTexel(0)) * step(0.001, fieldTexel(4)) * fieldTexel(7);
}
\n${e.slice(r)}`.replace(t,n)},bs={vertex:ys(On.vertex,`/*@fieldvertex*/`,`v_col = fieldColor(v_col, fieldValue(gl_VertexID));`),fragment:ys(On.fragment,`/*@fieldfragment*/`,`fieldNear = fieldKeep();`)},xs=-3e38;function Ss(e){let t=Math.floor(e.length/8),n=!1;for(let r=0;r<t;r+=3)if(sn(un(e,r))!==`solid`){n=!0;break}if(!n)return null;let r=[];for(let n=0;n+2<t;n+=3)sn(un(e,n))===`solid`&&r.push(n,n+1,n+2);return Int32Array.from(r)}const Cs=110574,ws=/* @__PURE__ */ new Float32Array(2601),Ts=/* @__PURE__ */ new Float32Array(2601),Es=/* @__PURE__ */ new WeakMap,Ds=e=>{let t=Es.get(e);return t||Es.set(e,t=/* @__PURE__ */ new Map),t},Os=2048,ks=e=>111320*Math.cos(e*B);var As=class extends la{kind=`shape`;stands=!0;g=null;prog=null;progI=null;progImp=null;imps=/* @__PURE__ */ new Map;impReady=!1;lazyWaiting=0;groundTex=null;groundLayers=0;groundTop=0;groundFree=[];growGround(e){let t=this.g;if(t){this.groundTex?.destroy(),this.groundLayers=Math.min(this.maxChunks,Math.max(e+1,this.groundLayers*2)),this.groundTex=t.texture({label:`solids ${this.id} ground`,width:51,height:51,layers:this.groundLayers,format:`r32f`,filter:`nearest`});for(let e of this.chunks.values())e.glayer>=0&&this.groundTex.write(e.ground,{layer:e.glayer})}}groundInto(e){e.ground===ws?e.glayer>=0&&(this.groundFree.push(e.glayer),e.glayer=-1):(e.glayer<0&&(e.glayer=this.groundFree.pop()??this.groundTop++,e.glayer>=this.groundLayers&&this.growGround(e.glayer)),this.groundTex.write(e.ground,{layer:e.glayer}))}chunks=/* @__PURE__ */ new Map;free=[];pending=/* @__PURE__ */ new Map;hoverKey=null;shapeBufs=/* @__PURE__ */ new Map;tintFn=null;noTint=null;tintAtlas=null;tintPx=/* @__PURE__ */ new Map;recTex=null;recRows=0;recTop=Os;recFree=[];recRemake=!1;tileRow=/* @__PURE__ */ new Float32Array;tilesLo=1/0;tilesHi=-1;groups=/* @__PURE__ */ new Map;lbuf=null;lt=/* @__PURE__ */ new Float32Array(16);ltw=(/* @__PURE__ */ new Float32Array(16)).fill(NaN);drawn={chunks:0,vertices:0};nowMs=0;constructor(e={}){super(e)}get maxChunks(){return Math.min(256,this.props.maxChunks??this.chunksByDefault)}chunksByDefault=96;async attach(e){await super.attach(e);let t=e.graphics;if(!t)return;this.g=t;let n=[{name:`ground`,kind:`2d-array`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`tints`,kind:`2d`,sample:`float`},{name:`picture`,kind:`2d`,sample:`float`}];this.prog=t.program({label:`solids ${this.id}`,wgsl:Bn,glsl:On,blocks:[`Frame`,`T`],textures:n,vertex:{stride:8,attributes:[{name:`a`,location:0,size:4,offset:0},{name:`b`,location:1,size:4,offset:4}]},depth:{write:!0,test:`less`},cull:`none`}),this.progI=t.program({label:`solids ${this.id} instances`,wgsl:Hn,glsl:An,blocks:[`Frame`,`T`,`S`],textures:[...n,Fs],instance:Ps,vertex:{stride:6,attributes:[{name:`v`,location:3,size:4,offset:0},{name:`vc`,location:4,size:1,offset:4},{name:`card`,location:5,size:1,offset:5}]},depth:{write:!0,test:`less`},cull:`none`}),this.groundLayers=1,this.groundTex=t.texture({label:`solids ${this.id} ground`,width:51,height:51,layers:this.groundLayers,format:`r32f`,filter:`nearest`});for(let e=this.maxChunks-1;e>=0;e--)this.free.push(e);this.tileRow=new Float32Array(this.maxChunks*2*4).fill(NaN),this.progImp=t.program({label:`solids ${this.id} impostors`,wgsl:Wn,glsl:Pn,blocks:[`Frame`,`T`,`S`,`IM`],textures:[...n,Fs,{name:`atlas`,kind:`2d`,sample:`float`},{name:`normals`,kind:`2d`,sample:`float`}],instance:Ps,depth:{write:!0,test:`less`},cull:`none`}),this.progImp.ready.then(()=>{this.impReady=!0},t=>e.world.error(t)),this.fieldProgram(),await Promise.all([this.prog.ready,this.progI.ready]);for(let[e,t]of this.pending)this.put(e,t);this.pending.clear()}detach(){for(let e of[...this.chunks.keys()])this.drop(e);this.prog?.destroy(),this.progI?.destroy(),this.progImp?.destroy(),this.groundTex?.destroy(),this.progF?.destroy(),this.progF=null,this.progFReady=!1;for(let e of this.imps.values()){e.block?.destroy();for(let t of e.baked?.targets??[])t.destroy();for(let t of e.baked?.buffers??[])t.destroy()}this.imps.clear(),this.progImp=null,this.impReady=!1;for(let e of this.shapeBufs.values())e.destroy();this.shapeBufs.clear();for(let e of this.variants.values())e.mesh?.destroy(),e.inst?.destroy(),e.params?.buf.destroy();this.variants.clear();for(let e of this.formulaProgs.values())e.destroy();this.formulaProgs.clear(),this.tintAtlas?.destroy(),this.tintAtlas=null,this.tintPx.clear(),this.noTint?.destroy(),this.noTint=null,this.recTex?.destroy(),this.recTex=null,this.recRows=0,this.recTop=Os,this.recFree=[],this.lbuf?.destroy(),this.lbuf=null,this.ltw.fill(NaN);for(let e of this.pics.values())typeof e==`object`&&e.destroy();this.pics.clear(),this.noPic?.destroy(),this.noPic=null,this.prog=this.progI=null,this.groundTex=null,this.groundTop=0,this.groundFree=[],this.g=null,this.free=[],super.detach()}put(e,t){let n=this.g;if(this.placesV++,!n||!this.prog)return this.pending.set(e,t),this;let r=this.chunks.get(e),i=r?r.glayer:-1;r&&(r.glayer=-1,this.release(r));let a=r?r.slot:this.free.pop();if(a===void 0)return this.attached?.world.error(new W(`E-DATA-001`,`solids '${this.id}': more than ${this.maxChunks} chunks of 500 m at once`,{where:this.id,fix:this.maxChunks<256?`maxChunks: up to 256`:`its things within 8 × 8 km, or split into layers by area`})),this;let o=typeof t.mesh==`function`?t.mesh:null,s=t.mesh&&!o?cn(t.mesh,8,un):null,c=s?s.solid:t.mesh,l=c&&!o?c.length/8:0,u=ks(t.origin[1]),d={key:e,slot:a,spec:t,born:r?r.born:t.fade===!1?-1e9:this.nowMs,n:l,vbuf:l?n.buffer(`vertex`,c,`${this.id} ${e}`):null,lazy:o,lazyPending:!1,parts:s?this.partsOf(s,`${e}`):null,fn:t.far?t.far.length/8:0,fbuf:t.far&&t.far.length>=8?n.buffer(`vertex`,t.far,`${this.id} ${e} far`):null,inst:[],rdata:/* @__PURE__ */ new Float32Array,rbase:0,rn:0,square:[0,0,0,0,0,0],tintRow:-1,tintRows:0,fv:null,top:t.free?1e5:Ms(t),ext:js(o?null:t.mesh,js(t.far,null)),tbuf:n.buffer(`uniform`,64,`${this.id} ${e} block`),t:/* @__PURE__ */ new Float32Array(16),tw:(/* @__PURE__ */ new Float32Array(16)).fill(NaN),ground:r?.ground??ws,glayer:i,groundSeen:r?r.groundSeen:-1,groundFull:r?.groundFull??!1,groundRead:r?r.groundRead:-1,groundTop:r?.groundTop??NaN,inView:r?.inView??!0,away:r?.away??0,waits:r?.waits??0,corner:[t.origin[0]+t.at[0]/u,t.origin[1]-t.at[1]/Cs],lodNow:r?.lodNow??[-1,-1]};return this.records(d,t.instances??[]),t.instances?.length&&(d.spec={...t,instances:t.instances.map(e=>({...e,data:void 0,records:void 0}))}),this.chunks.set(e,d),this.changed(),this}has(e){return this.chunks.has(e)||this.pending.has(e)}topIn(e,t,n,r){let i=null;for(let a of this.chunks.values()){if(a.spec.free)continue;let o=Math.min(500,a.spec.side??500),s=ks(a.spec.origin[1]),c=a.corner[0],l=a.corner[1];if(c>n||c+o/s<e||l<t||l-o/Cs>r)continue;let u=(Number.isFinite(a.groundTop)?a.groundTop:0)+a.top;(i===null||u>i)&&(i=u)}return i}reachOf(e){return this.chunks.get(e)?.ext??null}keys(){return[...this.chunks.keys(),...this.pending.keys()]}moveInstances(e,t,n){let r=this.chunks.get(e),i=r?.inst[t];if(!r||!i)return this;if(Math.floor(n.length/10)!==i.ni){let e=r.inst.map((e,i)=>i===t?{...r.spec.instances[i],data:n,records:void 0}:{...r.spec.instances[i],data:void 0,records:e.rec.slice(),stride:e.R});this.releaseSets(r),this.records(r,e)}else Ls(n,i.rec,i.R,r.slot),this.writeRecords(r,i.base,i.ni*i.R),i.eye=null,i.job=null;return this.changed(),this}records(e,t){let n=t.map(e=>{let t=e.records?Math.max(3,e.stride??3):3;return{x:e,R:t,n:e.records?Math.floor(e.records.length/(t*4)):Math.floor((e.data?.length??0)/10)}}).filter(e=>e.n>0),r=0;for(let e of n)r+=e.n*e.R;let i=new Float32Array(r*4);e.rdata=i,e.rn=r,e.rbase=r?this.allot(r):0,e.inst=[];let a=0;for(let{x:t,R:r,n:o}of n){let n=i.subarray(a*4,(a+o*r)*4);if(t.records){n.set(t.records.subarray(0,o*r*4));for(let t=0;t<o;t++)n[t*r*4+11]=e.slot}else Ls(t.data,n,r,e.slot);let s=(t.joints??[]).slice(0,8),c=s.filter(e=>e.formula).map(e=>`${e.part}:${e.formula}`).join(`|`),l={chunk:e,shapes:(Array.isArray(t.shape)?t.shape:[t.shape]).map(e=>Kn(e)?{imp:this.impOf(e)}:e&&e.length>=6?this.shapeLevel(e):null),ni:o,radius:t.radius??0,rec:n,R:r,base:a,eye:null,forms:new Set(Array.isArray(t.shape)?t.shape:[t.shape]).size,range:t.size?Qo(t.size,`solids '${this.id}' size`):null,lv:null,counts:[0,0,0],sorts:0,far:!1,joints:s,formula:c,moving:s.some(e=>e.formula||e.own!==void 0||(e.rate??0)!==0)||!!t.emitters?.length,job:null,lodNow:1,group:null,pic:!!this.props.imagery&&Zs(Array.isArray(t.shape)?t.shape:[t.shape]),drawn:-1,cast:-1,drawnAt:-1,castAt:-1};e.inst.push(l),this.join(l,this.setData(r,t,s)),a+=o*r}this.writeRecords(e,0,r)}allot(e){for(let t=0;t<this.recFree.length;t++){let[n,r]=this.recFree[t];if(!(r<e))return r===e?this.recFree.splice(t,1):this.recFree[t]=[n+e,r-e],n}let t=this.recTop;return this.recTop+=e,Math.ceil(this.recTop/Os)>this.recRows&&(this.recRemake=!0),t}unallot(e,t){if(t<=0)return;let n=this.recFree,r=0;for(;r<n.length&&n[r][0]<e;)r++;n.splice(r,0,[e,t]),r+1<n.length&&n[r][0]+n[r][1]===n[r+1][0]&&(n[r][1]+=n[r+1][1],n.splice(r+1,1)),r>0&&n[r-1][0]+n[r-1][1]===n[r][0]&&(n[r-1][1]+=n[r][1],n.splice(r,1),r--);let i=n.at(-1);i&&i[0]+i[1]===this.recTop&&(this.recTop=i[0],n.pop())}syncRecords(){let e=this.g,t=this.recTex&&this.recRows>8&&Math.ceil(this.recTop/Os)*3<this.recRows;if(this.recRemake||!this.recTex||t){let n=0;for(let[,e]of this.recFree)n+=e;if(n*2>this.recTop-Os){let e=Os;for(let t of this.chunks.values())t.rbase=e,e+=t.rn;this.recTop=e,this.recFree=[];for(let e of this.groups.values())e.dirty=!0,e.castDirty=!0}let r=Math.max(1,Math.ceil(this.recTop/Os)),i=!this.recTex||t?r:Math.max(r,Math.ceil(this.recRows*1.5));if(i>e.caps.maxTex){this.attached?.world.error(new W(`E-DATA-001`,`solids '${this.id}': its clones' records need ${i} rows of ${Os} texels, more than the graphics' ${e.caps.maxTex}`,{where:this.id,fix:`fewer clones in one layer (split it by area), or fewer texels a record`})),this.recRemake=!1;return}(i!==this.recRows||!this.recTex)&&(this.recTex?.destroy(),this.recTex=e.texture({label:`${this.id} records`,width:Os,height:i,format:`rgba32f`,filter:`nearest`}),this.recRows=i),this.recRemake=!1;for(let e of this.chunks.values())e.rn&&this.recWrite(e.rbase,e.rdata);this.tilesLo=0,this.tilesHi=this.maxChunks-1}}syncTiles(){if(!this.recTex||this.tilesHi<this.tilesLo)return;let e=this.tilesLo,t=this.tilesHi;this.recTex.write(this.tileRow.subarray(e*8,(t+1)*8),{x:e*2,y:0,width:(t-e+1)*2,height:1}),this.tilesLo=1/0,this.tilesHi=-1}tileOf(e){let t=e.t,n=this.tileRow,r=e.slot*2*4,i=[t[0],t[1],t[2],t[3],t[6],t[7],t[15],t[14]],a=!0;for(let e=0;e<8;e++)n[r+e]!==i[e]&&(a=!1,n[r+e]=i[e]);a||(e.slot<this.tilesLo&&(this.tilesLo=e.slot),e.slot>this.tilesHi&&(this.tilesHi=e.slot))}recWrite(e,t){let n=this.recTex,r=t.length/4;for(let i=0;i<r;){let a=e+i,o=Math.floor(a/Os),s=a-o*Os;if(s===0&&r-i>=Os){let e=Math.floor((r-i)/Os);n.write(t.subarray(i*4,(i+e*Os)*4),{x:0,y:o,width:Os,height:e}),i+=e*Os}else{let e=Math.min(Os-s,r-i);n.write(t.subarray(i*4,(i+e)*4),{x:s,y:o,width:e,height:1}),i+=e}}}writeRecords(e,t,n){!this.recTex||this.recRemake||n<=0||this.recWrite(e.rbase+t,e.rdata.subarray(t*4,(t+n)*4))}writeRecord(e,t,n,r,i){let a=this.chunks.get(e),o=a?.inst[t];if(!a||!o||n<0||n>=o.ni||r>=o.R)return this;let s=o.base+n*o.R+r;return a.rdata.set(i.slice(0,4),s*4),r===2&&(a.rdata[s*4+3]=a.slot),this.writeRecords(a,s,1),this.changed(),this}recordsOf(e,t){let n=this.chunks.get(e)?.inst[t];return n?{rec:n.rec,stride:n.R,n:n.ni}:null}setClock(e,t){return this.chunks.get(e)?.inst[t]?.group?.S[8]??0}clockS(){return this.clockMs/1e3}clockMs=0;setJointRate(e,t,n,r){let i=this.chunks.get(e)?.inst[t],a=i?.joints[n],o=i?.group;if(!i||!a||!o)return this;let s=this.clockS(),c=16+n*16,l=o.S.slice(),u=l[c+9],d=l[8];return u===r||(l[c+14]=l[c+14]+u*(s-d)-r*(s-d),l[c+9]=r,a.rate=r,i.moving=i.joints.some(e=>e.formula||e.own!==void 0||(e.rate??0)!==0),this.leave(i),this.join(i,l,!0),this.changed()),this}join(e,t,n=!1){let r=`${e.shapes.map(Js).join(`,`)}|${e.pic?e.chunk.key:``}|${Ys(t)}`,i=this.groups.get(r);if(!i){let a=t.slice();n||(a[8]=this.clockS()),a[12]=Math.max(.25,Math.min(1,this.detailSeen));let o=()=>({buf:null,n:0,cap:0}),s=()=>[1/0,1/0,1/0,-1/0,-1/0,-1/0];i={key:r,members:/* @__PURE__ */ new Set,shapes:e.shapes,S:a,sbuf:this.g.buffer(`uniform`,a,`${this.id} set`),sized:e.radius>0&&e.forms>1,joints:e.joints,formula:e.formula,moving:e.moving,pic:e.pic?e.chunk:null,lists:[o(),o(),o()],casts:[o(),o()],dirty:!0,castDirty:!0,box:s(),castBox:s(),clocky:0},this.groups.set(r,i)}i.members.add(e),e.group=i,i.dirty=!0,i.castDirty=!0}leave(e){let t=e.group;if(t&&(t.members.delete(e),e.group=null,t.dirty=!0,t.castDirty=!0,!t.members.size)){t.sbuf.destroy();for(let e of[...t.lists,...t.casts])e.buf?.destroy();this.groups.delete(t.key)}}listsOf(e,t){let n=t?e.casts:e.lists,r=e.shapes.some(e=>e?.imp),i=n.map(()=>0),a=n=>t?(e.sized?n<2:n===0)||!r?0:1:n;for(let n of e.members){let e=t?n.cast:n.drawn;if(!(e<0)){if(e<3)i[a(e)]+=n.ni;else for(let e=0;e<3;e++)i[a(e)]+=n.counts[e]}}let o=i.map(e=>new Float32Array(e)),s=i.map(()=>0);for(let n of e.members){let e=t?n.cast:n.drawn,r=n.chunk.rbase+n.base,i=n.R;if(e<0)continue;if(e<3){let t=a(e),c=o[t],l=s[t];for(let e=0;e<n.ni;e++)c[l+e]=r+e*i;s[t]+=n.ni;continue}let c=n.lv;for(let e=0;e<n.ni;e++){let t=a(c[e]);o[t][s[t]++]=r+e*i}}o.forEach((e,r)=>{let i=n[r],a=e.length;if(i.n=a,a){if(!i.buf||a>i.cap||a*3<i.cap){i.buf?.destroy(),i.cap=Math.ceil(a*1.25);let n=new Float32Array(i.cap);n.set(e),i.buf=this.g.buffer(`instance`,n,`${this.id} ${t?`casting`:`level`} ${r}`)}else i.buf.write(e)}})}drop(e){this.pending.delete(e),this.placesV++;let t=this.chunks.get(e);return t&&(this.release(t),this.chunks.delete(e),this.free.push(t.slot),this.changed()),this}framing=!1;changed(){this.framing||(this.attached?.world)?.invalidate?.()}release(e){this.dropField(e),e.glayer>=0&&this.groundFree.push(e.glayer);let t=this.pics.get(e.key);typeof t==`object`&&t.destroy(),this.pics.delete(e.key),this.tintPx.delete(e.key),e.vbuf?.destroy(),e.fbuf?.destroy(),this.dropParts(e.parts),e.parts=null,this.releaseSets(e),e.tbuf.destroy()}releaseSets(e){for(let t of e.inst)this.leave(t),this.letGoLevels(t);e.inst=[],this.unallot(e.rbase,e.rn),e.rn=0}setData(e,t,n){let r=/* @__PURE__ */ new Float32Array(168),i=t.size?Qo(t.size,`solids '${this.id}' size`):null,a={hinge:1,wheel:2,slider:3};return r.set([(t.radius??0)*2,+!!i,i?i[1]:0,i?i[2]:1e6,0,e,0,+!!t.groups,0,n.length,0,0,Math.max(.25,Math.min(1,this.detailSeen)),0,0,0],0),(t.emitters??[]).slice(0,2).forEach((e,t)=>r.set([e.at[0],e.at[1],e.at[2],e.life,e.rise,e.spread,e.grow,e.count,e.fade[0],e.fade[1],e.drift[0],e.drift[1]],144+t*12)),n.forEach((e,t)=>{let n=16+t*16,i=e.limits;r.set([e.pivot[0],e.pivot[1],e.pivot[2],a[e.kind],e.axis[0],e.axis[1],e.axis[2],e.radius??0,e.formula?2:+(e.rate!==void 0||e.own!==void 0),e.rate??0,i?i[0]:0,i?i[1]:0,e.part,e.own??-1,0,0],n)}),r}impOf(e){let t=this.imps.get(e);if(!t){let n=this.g,r={level:e,baked:null,block:null,failed:!1};t=r,this.imps.set(e,r),Yn(n,e,this.shapeBuf(e.shape)).then(t=>{if(this.g!==n)for(let e of t.targets)e.destroy();else r.baked=t,r.block=n.buffer(`uniform`,Float32Array.from([e.radius,e.centreY,e.views,e.size]),`${this.id} impostor`),(this.attached?.world)?.invalidate?.()},e=>{r.failed=!0,this.attached?.world.error(e)})}return t}shapeBuf(e){let t=this.shapeBufs.get(e);return t||(t=this.g.buffer(`vertex`,e,`${this.id} shape`),this.shapeBufs.set(e,t)),t}shapeLevel(e){let t=Ds(this.g),n=t.get(e);if(!n){let r=cn(e,6,dn),i=this.g;n=r?{buf:i.buffer(`vertex`,r.solid.length?r.solid:/* @__PURE__ */ new Float32Array(18),`shapes`),nv:r.solid.length/6,parts:this.partsOf(r,`shape`),refs:0,bytes:e.byteLength,used:0}:{buf:i.buffer(`vertex`,e,`shapes`),nv:e.length/6,parts:null,refs:0,bytes:e.byteLength,used:0},t.set(e,n)}return n.refs++,n}letGoLevels(e){let t=Ds(this.g),n=performance.now();for(let t of e.shapes)if(t&&!t.imp){let e=t;e.refs=Math.max(0,e.refs-1),e.refs||(e.used=n)}let r=0,i=[];for(let e of t)e[1].refs||(r+=e[1].bytes,i.push(e));if(!(r<=25165824)){i.sort((e,t)=>e[1].used-t[1].used);for(let[e,n]of i){if(r<=25165824)break;n.buf.destroy(),this.dropParts(n.parts),t.delete(e),r-=n.bytes}}}partsOf(e,t){let n=this.g,r=(e,r)=>n.buffer(`vertex`,e,`${this.id} ${t} ${r}`);return{lines:e.lines?{buf:r(e.lines,`lines`),n:e.lines.length}:null,ghost:e.ghost?{buf:r(e.ghost,`ghost`),n:e.ghost.length}:null,custom:[...e.custom].map(([e,t])=>({code:e,buf:r(t,`material ${e}`),n:t.length})),emit:e.emit?{buf:r(e.emit,`copies over time`),n:e.emit.length}:null}}dropParts(e){if(e){e.lines?.buf.destroy(),e.ghost?.buf.destroy(),e.emit?.buf.destroy();for(let t of e.custom)t.buf.destroy()}}variants=/* @__PURE__ */ new Map;variant(e){let t=String(e),n=this.variants.get(t);if(n)return n;let r=this.g;if(!r)return null;let i=[{name:`ground`,kind:`2d-array`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`tints`,kind:`2d`,sample:`float`},{name:`picture`,kind:`2d`,sample:`float`}],a={stride:8,attributes:[{name:`a`,location:0,size:4,offset:0},{name:`b`,location:1,size:4,offset:4}]},o=Ps,s={stride:6,attributes:[{name:`v`,location:3,size:4,offset:0},{name:`vc`,location:4,size:1,offset:4},{name:`card`,location:5,size:1,offset:5}]},c={depth:{write:!0,test:`less`}},l={mesh:{wgsl:Bn,glsl:On},inst:{wgsl:Hn,glsl:An}},u=[`Frame`,`T`],d;if(e===`lines`)c={...c,topology:`lines`};else if(e===`ghost`)c={blend:`alpha`,depth:{write:!1,test:`less`}};else if(e===`emit`)c={blend:`alpha`,depth:{write:!1,test:`less`}},l={mesh:{wgsl:gs,glsl:hs},inst:{wgsl:gs,glsl:hs}};else{let i=an(e);if(!i)return null;let a=Gi(i.values),o=`material ${e}`;try{l={mesh:$i(l.mesh,i.spec,a,o),inst:$i(l.inst,i.spec,a,o,3)}}catch(e){return this.attached?.world.error(e),this.variants.set(t,n={mesh:null,inst:null}),n}u=[`Frame`,`T`,`MP`],d={block:a,buf:r.buffer(`uniform`,a.size,`${this.id} ${o} params`),version:-1,m:i};let s=i.spec.blend??`none`;c=s===`none`?c:{blend:s,depth:{write:!1,test:`less`}}}n={mesh:e===`emit`?null:r.program({label:`solids ${this.id} ${t}`,...l.mesh,blocks:u,textures:i,vertex:a,cull:`none`,...c}),inst:r.program({label:`solids ${this.id} ${t} instances`,...l.inst,blocks:[...u.slice(0,2),`S`,...u.slice(2)],textures:[...i,Fs],instance:o,vertex:s,cull:`none`,...c}),params:d};for(let e of[n.mesh,n.inst])e?.ready.catch(e=>this.attached?.world.error(e));return this.variants.set(t,n),n}partDraws(e,t,n,r,i){let a=(e,a,o,s)=>{let c=e?r?e.inst:e.mesh:null;if(!c)return;let l=e.params?[...n.blocks,e.params.buf]:n.blocks;e.params&&e.params.version!==e.params.m.version&&(e.params.buf.write(e.params.block.pack(e.params.m.values)),e.params.version=e.params.m.version),(c.desc.blend&&c.desc.blend!==`none`?this.late:t).push({...n,blocks:l,program:c,vertices:a,count:o/i,pickProgram:s?()=>c:void 0})};e.lines&&a(this.variant(`lines`),e.lines.buf,e.lines.n,!1),e.ghost&&a(this.variant(`ghost`),e.ghost.buf,e.ghost.n,!0),e.emit&&r&&a(this.variant(`emit`),e.emit.buf,e.emit.n,!1);for(let t of e.custom)a(this.variant(t.code),t.buf,t.n,!0),!r&&an(t.code)?.animated&&(this.clockAt=Math.min(this.clockAt,Us(this.eyeXZ,n.bounds)))}clockAt=1/0;eyeXZ=[0,0];late=[];groundOf(e,t,n){let r=this.chunks.get(e);if(!r||!r.groundFull)return NaN;let i=Math.max(0,Math.min(49.999,t/10)),a=Math.max(0,Math.min(49.999,n/10)),o=Math.floor(i),s=Math.floor(a),c=i-o,l=a-s,u=r.ground,d=(e,t)=>u[t*51+e];return(d(o,s)*(1-c)+d(o+1,s)*c)*(1-l)+(d(o,s+1)*(1-c)+d(o+1,s+1)*c)*l}groundHolds(e,t){if(!t.groundFull)return!1;let n=ks(t.spec.origin[1]),r=t.ground,i=Math.min(51,Math.ceil(Math.min(500,t.spec.side??500)/10)+2),a=(i-1)/4;for(let o=0;o<i;o+=a)for(let s=0;s<i;s+=a){let i=Math.round(s),a=Math.round(o),c=e.world.heightAt(t.spec.origin[0]+(t.spec.at[0]+i*10)/n,t.spec.origin[1]-(t.spec.at[1]+a*10)/Cs);if(!(Math.abs(c-r[a*51+i])<.001))return!1}return!0}groundFrame=0;groundUnder(e,t){let n=null;for(let e of this.chunks.values())e.groundRead!==t&&(e.spec.free?(e.groundSeen=e.groundRead=t,e.groundFull=!0):(n??=[]).push(e));if(!n)return;let r=++this.groundFrame;for(let e of n)e.groundSeen!==t&&!e.waits&&(e.waits=r);let i=e=>(e.groundSeen===t?3:0)+(e.inView?e.away<1500?0:1:2),a=e=>(r-e.waits+1)/Math.max(200,e.away),o=(e,t)=>{let n=i(e);return n-i(t)||(n<2?a(t)-a(e):e.away-t.away)};n.sort(o);let s=performance.now(),c=this.props.groundMs??3,l=0;for(let r of n){if(i(r)!==0)break;this.groundHolds(e,r)?(r.groundSeen=t,r.waits=0):l>=3&&performance.now()-s>c||(r.groundSeen=r.groundRead=t,r.waits=0,this.readGround(e,r),l++)}let u=0;for(let r of n.filter(e=>e.groundRead!==t&&i(e)!==0).sort(o)){if((u||l)&&performance.now()-s>c)break;u++,r.groundSeen!==t&&this.groundHolds(e,r)?(r.groundSeen=t,r.waits=0):(r.groundSeen=r.groundRead=t,r.waits=0,this.readGround(e,r))}}readGround(e,t){let n=ks(t.spec.origin[1]),r=Ts,i=!0;r.set(t.ground);let a=Math.min(51,Math.ceil(Math.min(500,t.spec.side??500)/10)+2),o=-1/0;for(let s=0;s<a;s++)for(let c=0;c<a;c++){let a=t.spec.origin[0]+(t.spec.at[0]+c*10)/n,l=t.spec.origin[1]-(t.spec.at[1]+s*10)/Cs,u=e.world.heightAt(a,l);Number.isNaN(u)?(i=!1,t.groundFull||(r[s*51+c]=0)):r[s*51+c]=u,r[s*51+c]>o&&(o=r[s*51+c])}t.groundFull=i,t.groundTop=o;let s=!0;for(let e=0;e<r.length;e++)if(r[e]!==0){s=!1;break}s?t.ground=ws:(t.ground===ws&&(t.ground=/* @__PURE__ */ new Float32Array(2601)),t.ground.set(r)),this.groundInto(t)}placesV=0;cellsV=-1;cellsNow=[];countLayer=null;countedAt(e){let t=this.props.counted;if(!t)return!1;let n=e.observation;return!n||n.level===null?!1:we(t.level,n.levels,n.index,`${this.id} counted`)}cells(){if(this.cellsV===this.placesV)return this.cellsNow;this.cellsV=this.placesV;let e=Math.max(1,this.props.counted?.cell??1e3),t=/* @__PURE__ */ new Map,n=0,r=[...[...this.chunks.values()].map(e=>({sp:e.spec,sets:e.inst.map(e=>({shape:e.shapes,records:e.rec,stride:e.R}))})),...[...this.pending.values()].map(e=>({sp:e,sets:e.instances??[]}))];for(let{sp:i,sets:a}of r){let r=ks(i.origin[1]);n||=r;for(let o of a)for(let[a,s]of Ns(o)){let o=i.origin[0]+(i.at[0]+a)/r,c=i.origin[1]-(i.at[1]+s)/Cs,l=`${Math.floor(o*n/e)}_${Math.floor(c*Cs/e)}`,u=t.get(l);u||t.set(l,u={lon:0,lat:0,count:0}),u.lon+=o,u.lat+=c,u.count++}}return this.cellsNow=[...t.values()].map(e=>({lon:e.lon/e.count,lat:e.lat/e.count,count:e.count})),this.countLayer?.setProps?.({data:this.cellFeatures()}),this.cellsNow}cellFeatures(){return this.cellsNow.map((e,t)=>({id:`${this.id}-cell-${t}`,geometry:{type:`Point`,coordinates:[e.lon,e.lat]},props:{count:e.count}}))}companions(){let e=this.props.counted;if(!e)return[];let t=e.format??(e=>e<1e3?String(e):e<1e6?`${(e/1e3).toFixed(+(e<1e4))}k`:`${(e/1e6).toFixed(1)}M`);return this.countLayer??=ms({id:`${this.id}-counted`,data:this.cellFeatures(),text:e=>t(Number(e.props.count)),size:e.size??13,color:e.color??`#ffffff`,priority:e=>Number(e.props.count),occlude:!1,when:e=>this.target<=0||this.props.instances===!1||!this.countedAt(e)?!1:(this.cells(),!0)}),[this.countLayer]}frame(e){if(this.nowMs=e.loadNow??e.now,this.clockMs=e.now,this.sortStats.sets=0,this.sortStats.ms=0,this.clockAt=1/0,this.eyeXZ[0]=e.camera.eye[0],this.eyeXZ[1]=e.camera.eye[2],!this.g||!this.prog?.ready||!this.groundTex||!this.chunks.size)return[];this.syncSelection(e);let t=this.surf?this.surf.look(e.s):null;this.surfLookNow=t,t&&this.fieldLook(t);let n=performance.now(),r=!1,i=this.syncTints();this.groups.size?this.syncRecords():this.recTex&&(this.recTex.destroy(),this.recTex=null,this.recRows=0);let a=e.camera,o=e.world.groundVersion,s=e.world.exaggeration;this.groundUnder(e,o);let c=a.lat,l=z*Math.cos(c*B),u=ae(c),d=this.props.hover===!1?null:Kr(this.props.hover?.color??`#ffd166`),f=(this.props.fade??.25)*1e3,p=this.props.shadows!==!1,m=e.shadowMap,h=[],g=this.detailOf(e.s),[_,v]=(this.props.lod??[350,800]).map(e=>e*g),y=a.eye[0],b=a.eye[2],x=a.viewProj,S=0,C=0,w=2;if(this.lazyWaiting=0,g!==this.detailSeen){this.detailSeen=g;for(let e of this.chunks.values())for(let t of e.inst)t.eye=null,t.job=null;for(let e of this.groups.values())e.S[12]=Math.max(.25,Math.min(1,g)),e.sbuf.write(e.S)}let T=!this.countedAt(e.s)&&this.props.instances!==!1,E=p&&T?e.shadowView??null:null,D=this.props.windows?.lit,O=Math.max(0,Math.min(1,typeof D==`function`?D(e.s):D??.45)),k=this.hoverKey?this.hoverKey.slice(0,this.hoverKey.lastIndexOf(`:`)):null,A=this.hoverKey?Number(this.hoverKey.slice(this.hoverKey.lastIndexOf(`:`)+1)):-1;for(let e of this.groups.values())Ws(e.box),Ws(e.castBox),e.clocky=this.clockyOf(e);for(let o of this.chunks.values()){let c=ks(o.spec.origin[1]),g=o.corner[1],D=o.t;D[0]=l*((o.corner[0]-a.lon+540)%360-180)*B,D[1]=-l*(ae(g)-u),D[2]=l*B/c,D[3]=l*B/(Cs*Math.cos(g*B)),D[4]=s,D[5]=this.pickNo,D[6]=d&&o.key===k&&!this.selBy.get(o.key)?.has(A)?A:-1,D[7]=Math.min(1,Math.max(.001,(this.nowMs-o.born)/Math.max(1,f))),D[8]=d?d[0]:1,D[9]=d?d[1]:1,D[10]=d?d[2]:1,D[11]=o.slot,D[12]=O,D[13]=+!!this.props.windows?.interior,D[14]=o.tintRow,D[15]=o.glayer;let j=Math.min(500,o.spec.side??500),M=o.ext,N=1/0,P=-1/0;for(let e=0;e<o.ground.length;e+=13){let t=o.ground[e];t<N&&(N=t),t>P&&(P=t)}Number.isFinite(N)||(N=0,P=0);let F=(e,t,n,r,i,a)=>{let o=D[0]+e*D[2],s=D[0]+n*D[2],c=D[1]+t*D[3],l=D[1]+r*D[3];return[Math.min(o,s),i,Math.min(c,l),Math.max(o,s),a,Math.max(c,l)]},I=N*s-40,ee=P*s+o.top+5,L=F(0,0,j,j,I,ee),R=M?F(M[0]-2,M[2]-2,M[3]+2,M[5]+2,o.spec.free?M[1]-2:I,o.spec.free?M[4]+2:ee):L,te=Ie(x,R),ne=o.inst.length>0&&Ie(x,L),re=te||ne;o.square=L;let z=e=>Math.hypot(Math.max(e[0]-y,0,y-e[3]),Math.max(e[2]-b,0,b-e[5])),ie=z(R),V=M?z(L):ie,H=o.inst.length?V:ie;o.inView=re,o.away=V;let oe=1+(this.props.pixels?.margin??.2),se=Vs(ie,o.lodNow[0],_,v,oe,!0),ce=Vs(H,o.lodNow[1],_,v,oe,!0);if(o.lodNow[0]=se,o.lodNow[1]=ce,o.lazy&&!o.vbuf&&V<v*1.15&&this.lazyWaiting++,o.lazy&&!o.vbuf&&!o.lazyPending&&V<v*1.15&&w-->0){let t=e=>{if(o.lazyPending=!1,this.chunks.get(o.key)!==o||o.vbuf||!this.g)return;o.ext=js(e,o.ext);let t=cn(e,8,un),n=t?t.solid:e;if(this.dropParts(o.parts),o.parts=t?this.partsOf(t,o.key):null,o.n=n.length/8,o.vbuf=o.n?this.g.buffer(`vertex`,n,`${this.id} ${o.key}`):null,this.surf){let t=this.fieldOf(o);t.mesh=e,t.nearV=-1}!o.n&&!o.parts&&(o.lazy=null),(this.attached?.world)?.invalidate?.()};try{let n=o.lazy();n instanceof Promise?(o.lazyPending=!0,n.then(t,t=>{o.lazyPending=!1,o.lazy=null,e.world.error(t)})):t(n)}catch(t){o.lazy=null,e.world.error(t)}}else o.lazy&&o.vbuf&&V>v*2&&(o.vbuf.destroy(),o.vbuf=null,o.n=0,this.dropParts(o.parts),o.parts=null,o.fv?.near&&(o.fv.near.tex.destroy(),o.fv.near=null,o.fv.nearV=-1));t&&this.fieldSync(o,t,n)&&(r=!0);let le=!0;for(let e=0;e<16;e++)D[e]!==o.tw[e]&&(le=!1,o.tw[e]=D[e]);le||o.tbuf.write(D),o.inst.length&&this.tileOf(o);let ue=[this.groundTex,m,i,this.pictureOf(o)],U=[e.frame,o.tbuf],de=this.prog;if(re&&S++,te){let e=(se===2||!o.vbuf)&&o.fbuf,n=e?o.fbuf:o.vbuf,r=e?o.fn:o.n,i=t&&this.progF&&this.progFReady?e?o.fv?.far:o.fv?.near:null,a=i&&i.n===r?i:null;a&&this.fieldHead(a,t);let s=a?this.progF:de;n&&r&&(h.push({program:s,blocks:U,textures:a?[...ue,a.tex]:ue,vertices:n,count:r,pickProgram:()=>s,bounds:R}),C+=r),o.parts&&!e&&this.partDraws(o.parts,h,{blocks:U,textures:ue,bounds:R},!1,8)}if(p){let e=o.fbuf??o.vbuf,t=o.fbuf?o.fn:o.n;e&&t&&h.push({program:de,blocks:U,textures:ue,vertices:e,count:t,pickProgram:()=>de,cast:p,bounds:R,shadowOnly:!0})}let fe=!!E&&Ie(E,L);for(let e of o.inst){let t=e.group;ne&&T&&(e.lodNow=ce),T&&(ne||fe)&&t.sized&&this.levelsOf(e,o,a,H);let n=t.sized?e.far?2:e.lv?3:e.lodNow:ce,r=ne&&T?n:-1,i=fe?n:-1;if((r!==e.drawn||r===3&&e.drawnAt!==e.sorts)&&(e.drawn=r,e.drawnAt=e.sorts,t.dirty=!0),(i!==e.cast||i===3&&e.castAt!==e.sorts)&&(e.cast=i,e.castAt=e.sorts,t.castDirty=!0),r>=0&&(Gs(t.box,L),t.moving||t.clocky)){let n=r<3?1<<r:+!!e.counts[0]|(e.counts[1]?2:0)|(e.counts[2]?4:0);(t.moving&&n&3||n&t.clocky)&&(this.clockAt=Math.min(this.clockAt,Us(this.eyeXZ,L)))}i>=0&&Gs(t.castBox,L)}}this.groups.size&&this.syncTiles();let j=this.recTex,M=this.progI,N=j&&this.groups.size?this.layerBlock(s,d,O):null;if(j&&N)for(let t of this.groups.values()){t.dirty&&=(this.listsOf(t,!1),!1),E&&t.castDirty&&(this.listsOf(t,!0),t.castDirty=!1);let n=[this.groundTex,m,i,t.pic?this.pictureOf(t.pic):this.noPicture(),j],r=[e.frame,N,t.sbuf],a=this.setProgram(t)??M;for(let e=0;e<3;e++){let i=t.lists[e];if(!i.n||!i.buf)continue;let o=this.levelAt(t.shapes,e);if(o){if(o.imp){let e=this.impostorCall(o.imp,r,n,t.box,i.buf,i.n);e&&(h.push(e),C+=6*i.n)}else o.nv&&h.push({program:a,blocks:r,textures:n,instances:i.buf,vertices:o.buf,count:o.nv,instanceCount:i.n,pickProgram:()=>a,bounds:t.box}),o.parts&&this.partDraws(o.parts,h,{blocks:r,textures:n,instances:i.buf,instanceCount:i.n,bounds:t.box},!0,6),C+=o.nv*i.n}}if(E)for(let e=0;e<2;e++){let i=t.casts[e];if(!i.n||!i.buf)continue;let o=this.levelAt(t.shapes,e===0?1:t.shapes.length-1);if(o){if(o.imp){let e=this.impostorCall(o.imp,r,n,t.castBox,i.buf,i.n);e&&h.push({...e,cast:p,shadowOnly:!0})}else o.nv&&h.push({program:a,blocks:r,textures:n,instances:i.buf,vertices:o.buf,count:o.nv,instanceCount:i.n,pickProgram:()=>a,cast:p,bounds:t.castBox,shadowOnly:!0})}}}return this.drawn={chunks:S,vertices:C},this.fieldMore=r,this.late.length&&(h.push(...this.late),this.late=[]),h}layerBlock(e,t,n){let r=this.lt;r[4]=e,r[5]=this.pickNo,r[6]=-1,r[7]=1,r[8]=t?t[0]:1,r[9]=t?t[1]:1,r[10]=t?t[2]:1,r[12]=n,r[13]=+!!this.props.windows?.interior;let i=this.lbuf??=this.g.buffer(`uniform`,64,`${this.id} layer block`),a=!0;for(let e=0;e<16;e++)r[e]!==this.ltw[e]&&(a=!1,this.ltw[e]=r[e]);return a||i.write(r),i}clockyOf(e){let t=0;for(let n=0;n<3;n++){let r=this.levelAt(e.shapes,n)?.parts;r&&(r.emit||r.custom.some(e=>an(e.code)?.animated))&&(t|=1<<n)}return t}lodByPixels(e,t,n,r=null){let i=Math.max(1,n.eye[1]-n.groundY),a=Math.hypot(t,i),o=e*2*n.height/(2*Math.tan(n.fov/2)*a),s=r?Math.min(r[2],Math.max(r[1],o)):o,[c,l,u]=this.edges();return s*u>c?0:s*u>l?1:2}edges(){let e=this.props.pixels??{},t=Math.max(.25,Math.min(1,this.detailSeen));return[(e.full??120)/t,(e.simple??25)/t,1+(e.margin??.2)]}detailSeen=1;sortStats={sets:0,ms:0};levelChanges=0;formulaProgs=/* @__PURE__ */ new Map;setProgram(e){if(!e.formula||!this.g)return null;let t=this.formulaProgs.get(e.formula);if(!t){let n={wgsl:Hn,glsl:An.vertex},r=Is(e.joints,`solids '${this.id}'`),i=[{name:`ground`,kind:`2d-array`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`tints`,kind:`2d`,sample:`float`},{name:`picture`,kind:`2d`,sample:`float`},Fs];t=this.g.program({label:`solids ${this.id} instances (formulas)`,wgsl:n.wgsl.replace(`/*@jointFormula*/`,r.wgsl),glsl:{vertex:n.glsl.replace(`/*@jointFormula*/`,r.glsl),fragment:An.fragment},blocks:[`Frame`,`T`,`S`],textures:i,instance:Ps,vertex:{stride:6,attributes:[{name:`v`,location:3,size:4,offset:0},{name:`vc`,location:4,size:1,offset:4},{name:`card`,location:5,size:1,offset:5}]},depth:{write:!0,test:`less`},cull:`none`});let a=e.formula;t.ready.then(()=>{this.formulaReady.add(a),(this.attached?.world)?.invalidate?.()},e=>this.attached?.world.error(e)),this.formulaProgs.set(e.formula,t)}return this.formulaReady.has(e.formula)?t:null}formulaReady=/* @__PURE__ */ new Set;levelAt(e,t){for(let n=Math.min(t,e.length-1);n>=0;n--){let t=e[n];if(!t)return null;if(!t.imp||t.imp.baked&&this.impReady)return t}return null}impostorCall(e,t,n,r,i,a){let o=this.progImp;return!o||!e.baked||!e.block?null:{program:o,blocks:[...t,e.block],textures:[...n,e.baked.color,e.baked.normals],instances:i,count:6,instanceCount:a,pickProgram:()=>o,bounds:r}}levelsOf(e,t,n,r){if(this.lodByPixels(e.radius*1.6,r,n,e.range)===2){e.far||(e.far=!0,e.job=null,e.eye=null,e.lv&&(e.lv.fill(2),e.counts=[0,0,e.ni],e.sorts++));return}e.far=!1;let i=t.t,a=(n.eye[0]-i[0])/i[2],o=(n.eye[2]-i[1])/i[3],s=n.eye[1]-n.groundY;!e.job&&(!e.eye||Math.hypot(a-e.eye[0],o-e.eye[1],s-e.eye[2])>Math.max(3,s*.05))&&(e.eye=[a,o,s],e.job={eye:[a,o,s],k:e.radius*2*n.height/(2*Math.tan(n.fov/2)),edges:this.edges(),next:0,lv:new Uint8Array(e.ni),counts:[0,0,0]}),e.job&&this.sortStep(e)}sortStep(e){let t=e.job,n=performance.now(),r=this.clockMs;if(Rs!==r&&(Rs=r,zs=0),zs>=1.5)return;let[i,a,o]=t.eye,s=t.k,[c,l,u]=t.edges,d=e.rec,f=e.R*4,p=e.ni,m=e.range,h=e.lv&&e.lv.length===p?e.lv:null,g=t.lv,_=t.next;for(;_<p&&!(!(_&127)&&_>t.next&&zs+performance.now()-n>=1.5);_++){let e=_*f,n=d[e]-i,r=d[e+1]-a,p=o-d[e+2],v=s*Math.max(d[e+4],d[e+5])/Math.sqrt(Math.max(n*n+r*r+p*p,1e-6)),y=Vs(m?Math.min(m[2],Math.max(m[1],v)):v,h?h[_]:-1,c,l,u,!1);g[_]=y,t.counts[y]++}if(t.next=_,_>=p&&zs+performance.now()-n<1.5){if(h)for(let e=0;e<p;e++)h[e]!==g[e]&&this.levelChanges++;e.lv=g,e.counts=t.counts,e.job=null,e.sorts++,this.sortStats.sets++}let v=performance.now()-n;zs+=v,this.sortStats.ms+=v}surf=null;progF=null;progFReady=!1;fieldMore=!1;clonesLook=``;fieldWrites=0;surface(e){if(this.surf===e)return this;for(let e of this.chunks.values())this.dropField(e);return this.surf=e,this.clonesLook=``,e?this.fieldProgram():(this.progF?.destroy(),this.progF=null,this.progFReady=!1),this.tintPx.clear(),(this.attached?.world)?.invalidate?.(),this}get surfaceOn(){return this.surf}meshVertices(){let e={};for(let[t,n]of this.chunks)e[t]=n.spec.mesh instanceof Float32Array?Math.floor(n.spec.mesh.length/8):null;for(let[t,n]of this.pending)e[t]=n.mesh instanceof Float32Array?Math.floor(n.mesh.length/8):null;return e}surfaceStats(){let e=0,t=0;for(let n of this.chunks.values())for(let r of[n.fv?.near,n.fv?.far])r&&(e++,t+=r.tex.desc.width*r.tex.desc.height*4);return{textures:e,bytes:t,writes:this.fieldWrites,pending:this.fieldMore}}fieldProgram(){let e=this.g;if(!e||this.progF||!this.surf)return;let t=[{name:`ground`,kind:`2d-array`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`tints`,kind:`2d`,sample:`float`},{name:`picture`,kind:`2d`,sample:`float`},{name:`fieldv`,kind:`2d`,sample:`unfilterable`}],n=this.progF=e.program({label:`solids ${this.id} field`,wgsl:vs,glsl:bs,blocks:[`Frame`,`T`],textures:t,vertex:{stride:8,attributes:[{name:`a`,location:0,size:4,offset:0},{name:`b`,location:1,size:4,offset:4}]},depth:{write:!0,test:`less`},cull:`none`});n.ready.then(()=>{this.progF===n&&(this.progFReady=!0,(this.attached?.world)?.invalidate?.())},e=>this.attached?.world.error(e))}fieldOf(e){return e.fv??={near:null,far:null,nearV:-1,farV:-1,clonesV:-1,clones:null,mesh:null,loading:!1}}dropField(e){let t=e.fv;t&&(t.near?.tex.destroy(),t.far?.tex.destroy(),e.fv=null,t.clones&&this.tintPx.delete(e.key))}nearMeshOf(e){if(e.spec.mesh instanceof Float32Array)return e.spec.mesh;let t=this.fieldOf(e);if(t.mesh)return t.mesh;if(!e.lazy||t.loading)return null;let n;try{n=e.lazy()}catch(e){return this.attached?.world.error(e),null}return n instanceof Promise?(t.loading=!0,n.then(n=>{t.loading=!1,e.fv===t&&(t.mesh=n,t.nearV=-1,t.farV=-1,(this.attached?.world)?.invalidate?.())},e=>{t.loading=!1,this.attached?.world.error(e)}),null):(t.mesh=n,n)}fieldSync(e,t,n){let r=this.surf,i=r.version,a=this.fieldOf(e),o=!!e.vbuf&&a.nearV!==i,s=!!e.fbuf&&a.farV!==i,c=e.inst.length>0&&a.clonesV!==i;if(!o&&!s&&!c)return!1;if(performance.now()-n>6)return!0;let l=(t,n=null)=>({key:e.key,spec:e.spec,mesh:t,clones:n,near:()=>this.nearMeshOf(e)});try{if(s&&(a.farV=i,a.far=this.fieldTex(e,a.far,r.values(l(e.spec.far??null),`far`),e.spec.far,`far`,null,t)),o){let n=this.nearMeshOf(e);n?(a.nearV=i,a.near=this.fieldTex(e,a.near,r.values(l(n),`near`),n,`near`,Ss(n),t),e.lazy&&(a.mesh=null)):a.loading||(a.nearV=i)}if(c){a.clonesV=i;let t=[];for(let n of e.inst)for(let e=0;e<n.ni;e++){let r=e*n.R*4;t.push({id:n.rec[r+3],x:n.rec[r],z:n.rec[r+1],h:n.rec[r+2]})}a.clones=r.values(l(null,t),`clones`),this.tintPx.delete(e.key)}}catch(e){this.attached?.world.error(e)}return!1}fieldTex(e,t,n,r,i,a,o){if(!n||!this.g)return t?.tex.destroy(),null;let s=Math.floor(r.length/8);if(n.a.length!==s||n.b&&n.b.length!==s)throw new W(`E-DATA-001`,`surface field '${this.surf?.id}' on '${this.id}': chunk '${e.key}' has ${s} vertices in its ${i===`near`?`whole`:`far`} mesh, the values ${n.b&&n.b.length!==s?n.b.length:n.a.length}`,{where:this.surf?.id,fix:`one value a vertex, in the mesh's own order (layer.meshVertices() gives each chunk's count), or a function of the thing and its facet`});let c=a?a.length:s,l=Math.max(1,Math.ceil(c/_s)),u=!!n.b,d=e=>{let t=new Float32Array(l*_s).fill(xs);for(let n=0;n<c;n++){let r=e[a?a[n]:n];t[n]=Number.isFinite(r)?r:xs}return t},f=d(n.a),p=n.b?d(n.b):null,m=(e,t)=>{if(!t||e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(e[n]!==t[n])return!1;return!0};if(t&&t.n===c&&t.rows===l&&t.two===u)return m(f,t.a)||(t.tex.write(f,{x:0,y:1,width:_s,height:l}),t.a=f,this.fieldWrites++),p&&!m(p,t.b)&&(t.tex.write(p,{x:0,y:1+l,width:_s,height:l}),t.b=p,this.fieldWrites++),t;t?.tex.destroy();let h=this.g.texture({label:`${this.surf.id} ${e.key} ${i}`,width:_s,height:1+l*(u?2:1),format:`r32f`,filter:`nearest`}),g={tex:h,n:c,rows:l,two:u,a:f,b:p,head:(/* @__PURE__ */ new Float32Array(8)).fill(NaN),lookV:-1};return h.write(f,{x:0,y:1,width:_s,height:l}),this.fieldWrites++,p&&(h.write(p,{x:0,y:1+l,width:_s,height:l}),this.fieldWrites++),this.fieldHead(g,o),g}fieldHead(e,t){let n=[1,t.lo,t.hi,t.min,t.mix,e.two?t.f:0,e.rows,+!!t.subject];if(e.lookV!==t.version){let r=new Float32Array(_s);r.set(n),r.set(t.ramp.subarray(0,256),8),e.tex.write(r,{x:0,y:0,width:_s,height:1}),e.head.set(n),e.lookV=t.version,this.fieldWrites++;return}let r=!0;for(let t=0;t<8;t++)if(e.head[t]!==Math.fround(n[t])){r=!1;break}r||(e.head.set(n),e.tex.write(e.head,{x:0,y:0,width:8,height:1}))}fieldLook(e){let t=`${e.version}|${e.mix.toFixed(3)}|${Math.round(e.f*32)}|${e.min}`;if(t!==this.clonesLook){this.clonesLook=t;for(let e of this.chunks.values())e.fv?.clones&&this.tintPx.delete(e.key)}}cloneTint(e,t){let n=e.fv?.clones,r=this.surf?this.surfLookNow:null;if(!n||!r||r.mix<=0)return null;let i=n.a[t],a=n.b?.[t],o=i??NaN;if(a!==void 0&&Number.isFinite(a)&&(o=Number.isFinite(o)?o+(a-o)*r.f:a),!Number.isFinite(o)||o<r.min)return null;let s=Math.round(Math.max(0,Math.min(1,(o-r.lo)/(r.hi-r.lo||1)))*255),c=r.ramp[s];return[c>>16&255,c>>8&255,c&255,Math.round(r.mix*255)]}surfLookNow=null;tint(e){return this.tintFn=e,this.tintPx.clear(),(this.attached?.world)?.invalidate?.(),this}solidsBase(e){let t=this.g;return!t||!this.groundTex?null:{g:t,textures:[this.groundTex,e.shadowMap,this.noTintTex(),this.noPicture()]}}noPic=null;pics=/* @__PURE__ */ new Map;noPicture(){let e=this.g;return this.noPic??=(()=>{let t=e.texture({label:`no picture`,width:1,height:1,format:`rgba8`,filter:`nearest`});return t.write(/* @__PURE__ */ new Uint8Array(4)),t})()}pictureOf(e){let t=this.props.imagery;if(!t)return this.noPicture();let n=this.pics.get(e.key);if(n&&n!==`loading`&&n!==`none`)return n;if(!n){let n=e.spec.mesh instanceof Float32Array?e.spec.mesh:null,r=!1;if(n){for(let e=0;e<n.length/8;e+=3)if(un(n,e)===Zt.imagery){r=!0;break}}if(!r){for(let t of e.spec.instances??[])for(let e of Array.isArray(t.shape)?t.shape:[t.shape])if(e instanceof Float32Array)for(let t=0;t<e.length/6&&!r;t+=3)dn(e,t)===Zt.imagery&&(r=!0)}this.pics.set(e.key,r?`loading`:`none`),r&&this.paint(e,t).then(t=>{this.chunks.get(e.key)===e&&t?(this.pics.set(e.key,t),(this.attached?.world)?.invalidate?.()):t?.destroy()},t=>{this.pics.set(e.key,`none`),this.attached?.world.error(t)})}return this.noPicture()}async paint(e,t){if(typeof OffscreenCanvas>`u`||!this.g||!this.attached)return null;let n=Math.min(500,e.spec.side??500),r=t.size??512,i=Math.max(t.source.minZoom,Math.min(t.source.maxZoom,t.zoom??17)),a=ks(e.corner[1]),o=e.corner[0],s=e.corner[1],c=o+n/a,l=s-n/Cs,u=new OffscreenCanvas(r,r),d=u.getContext(`2d`);if(!d)return null;let[f,p]=ce(o,s,i),[m,h]=ce(c,l,i),g=new OffscreenCanvas(256,256),_=g.getContext(`2d`,{willReadFrequently:!1});for(let e=p;e<=h;e++)for(let n=f;n<=m;n++){if(_.clearRect(0,0,256,256),!await t.source.draw(_,{z:i,x:n,y:e},{loader:this.attached.loader,priority:`later`,prefetch:!1}))continue;let a=se(i,n,e),u=e=>(e-o)/(c-o)*r,f=e=>(s-e)/(s-l)*r;d.drawImage(g,u(a[0]),f(a[3]),u(a[2])-u(a[0]),f(a[1])-f(a[3]))}if(!this.g)return null;let v=this.g.texture({label:`${this.id} ${e.key} picture`,width:r,height:r,format:`rgba8`,filter:`linear`});return v.writeImage(u),v}syncTints(){let e=this.g,t=!!this.surfLookNow&&this.surfLookNow.mix>0&&[...this.chunks.values()].some(e=>e.fv?.clones);if(!this.tintFn&&!this.selBy.size&&!t){(this.tintAtlas||this.tintPx.size)&&(this.tintAtlas?.destroy(),this.tintAtlas=null,this.tintPx.clear(),this.tintRows=0,this.tintTop=0);for(let e of this.chunks.values())e.tintRow=-1;return this.noTintTex()}let n=[],r=!this.tintAtlas;for(let e of this.chunks.values()){if(!this.tintFn&&!this.selBy.has(e.key)&&!(t&&e.fv?.clones)){this.tintPx.delete(e.key),e.tintRow=-1;continue}if(this.tintPx.get(e.key))continue;let i=this.tintPixels(e);this.tintPx.set(e.key,i),n.push(e);let a=i.length/1024;e.tintRow>=0&&e.tintRows===a||(this.tintTop+a<=this.tintRows?(e.tintRow=this.tintTop,e.tintRows=a,this.tintTop+=a):r=!0)}for(let e of[...this.tintPx.keys()])this.chunks.has(e)||this.tintPx.delete(e);if(r){let t=0;for(let e of this.tintPx.values())t+=e.length/1024;this.tintAtlas?.destroy(),this.tintRows=Math.max(1,Math.ceil(t*1.5)),this.tintTop=0,this.tintAtlas=e.texture({label:`${this.id} tints`,width:256,height:this.tintRows,format:`rgba8`,filter:`nearest`});for(let e of this.chunks.values()){let t=this.tintPx.get(e.key);t?(e.tintRow=this.tintTop,e.tintRows=t.length/1024,this.tintTop+=e.tintRows,this.tintAtlas.write(t,{y:e.tintRow,width:256,height:e.tintRows})):e.tintRow=-1}}else for(let e of n)this.tintAtlas.write(this.tintPx.get(e.key),{y:e.tintRow,width:256,height:e.tintRows});return this.tintAtlas}tintRows=0;tintTop=0;noTintTex(){let e=this.g;return this.noTint??=(()=>{let t=e.texture({label:`no tint`,width:1,height:1,format:`rgba8`,filter:`nearest`});return t.write(/* @__PURE__ */ new Uint8Array(4)),t})()}tintPixels(e){let t=this.selBy.get(e.key),n=0;for(;e.spec.info?.(n);)n++;if(t)for(let e of t)n=Math.max(n,e+1);let r=e.fv?.clones?.a;r&&(n=Math.max(n,r.length));let i=Math.max(1,Math.ceil(n/256)),a=new Uint8Array(256*i*4);if(r)for(let t=0;t<r.length;t++){let n=this.cloneTint(e,t);n&&a.set(n,t*4)}if(this.tintFn)for(let t=0;t<n;t++){let n=e.spec.info?.(t),r=n?this.tintFn(n,e.key,t):null;r&&a.set([r[0]*255,r[1]*255,r[2]*255,r[3]*255],t*4)}if(t){let e=this.selLook;for(let n of t)a.set(e,n*4)}return a}selKey=``;selBy=/* @__PURE__ */ new Map;selLook=null;syncSelection(e){let t=this.props.selected,n=e.s.user.selection;if(t===!1||!n.length){if(this.selBy.size){for(let e of this.selBy.keys())this.tintPx.delete(e);this.selBy=/* @__PURE__ */ new Map}this.selKey=``;return}let r=Kr(t?.color??`#38bdf8`),i=[r[0]*255,r[1]*255,r[2]*255,Math.max(0,Math.min(1,t?.mix??.65))*255].map(Math.round),a=`${i.join(`,`)}|${n.join(``)}`;if(a===this.selKey)return;let o=this.selLook?.join(`,`)!==i.join(`,`);this.selKey=a,this.selLook=i;let s=/* @__PURE__ */ new Map,c=e.world;for(let e of n){let t=e.lastIndexOf(`:`),n=Number(e.slice(t+1));if(t<1||!Number.isInteger(n)||n<0)continue;let r=c.selectedBy?.(e);if(r!==void 0&&r!==this.id)continue;let i=e.slice(0,t),a=s.get(i);a||s.set(i,a=/* @__PURE__ */ new Set),a.add(n)}for(let e of/* @__PURE__ */ new Set([...this.selBy.keys(),...s.keys()])){let t=this.selBy.get(e),n=s.get(e);(o||!t||!n||t.size!==n.size||[...t].some(e=>!n.has(e)))&&this.tintPx.delete(e)}this.selBy=s}fading(){let e=(this.props.fade??.25)*1e3;for(let t of this.chunks.values())if(this.nowMs-t.born<e)return!0;return!1}clockRate(){let e=this.clockAt;if(e===1/0)return 0;let[t,n]=this.props.lod??[350,800];return e<t?1/0:e<n?30:10}sorting(){for(let e of this.chunks.values())for(let t of e.inst)if(t.job)return!0;return!1}animating(e){return this.fading()||this.sorting()||this.groundWaits()||this.fieldMore}groundWaits(){let e=this.attached?.world.groundVersion;for(let t of this.chunks.values())if(t.groundRead!==e&&!t.spec.free)return!0;return!1}wantsFrames(e){return this.animating(e)?1/0:this.clockRate()}status(){let e=super.status();if(this.shown<=0&&this.target<=0)return e;this.pending.size&&e.pending.push(`${this.pending.size} chunk(s) waiting for the graphics`);let t=(this.props.fade??.25)*1e3,n=this.attached?.world.groundVersion;for(let n of this.chunks.values())if(this.nowMs-n.born<t){e.pending.push(`chunks appearing`);break}for(let t of this.chunks.values())if(t.groundRead!==n){e.pending.push(`reading the ground`);break}this.lazyWaiting>0&&e.pending.push(`${this.lazyWaiting} near chunk(s) building their whole mesh`);{let t=0;for(let e of this.chunks.values())for(let n of e.inst)n.job&&t++;t&&e.pending.push(`${t} instance set(s) choosing their levels`)}return this.surf&&(this.fieldMore||this.progF&&!this.progFReady)&&e.pending.push(`surface field '${this.surf.id}' putting its values`),e.ready=!e.pending.length,e}pickInfo(e){let t=Math.floor(e/65536),n=e%65536;for(let r of this.chunks.values())if(r.slot===t){let t=r.spec.info?.(n);if(!t)return null;let i={...t,chunk:r.key,local:n};return{element:this.id,id:`${r.key}:${n}`,props:t.props,d:i,index:e}}return null}hovered(e){this.hoverKey=e&&e.size?[...e][0]:null}placeOf(e){let t=e.d;if(!t||t.chunk===void 0)return null;let n=this.chunks.get(t.chunk);if(!n)return null;let r=ks(n.spec.origin[1]),i=this.attached?.world,a=i?.exaggeration??1,o=(e,t)=>[n.spec.origin[0]+(n.spec.at[0]+e)/r,n.spec.origin[1]-(n.spec.at[1]+t)/Cs],s=(e,t)=>{let n=i?.heightAt(e,t)??0;return Number.isNaN(n)?0:n*a};for(let e of n.inst){let r=e.rec,i=e.R*4;for(let a=0;a+i<=r.length;a+=i){if(r[a+3]!==t.local)continue;let[i,c]=o(r[a],r[a+1]);return{at:[i,c,n.spec.free?r[a+2]:s(i,c)+r[a+2]],turn:r[a+7],scale:[r[a+4],r[a+5],r[a+6]],radius:e.radius>0?e.radius*Math.max(r[a+4],r[a+6]):void 0}}}let c=n.spec.mesh instanceof Float32Array?n.spec.mesh:n.spec.far;if(c){let e=0,n=0,r=0,i=1/0,a=-1/0,l=1/0,u=-1/0;for(let o=0;o+8<=c.length;o+=8){if(c[o+7]%65536!==t.local)continue;let s=c[o],d=c[o+2];e+=s,n+=d,r++,i=Math.min(i,s),a=Math.max(a,s),l=Math.min(l,d),u=Math.max(u,d)}if(r){let[t,c]=o(e/r,n/r);return{at:[t,c,s(t,c)],turn:0,scale:[1,1,1],radius:Math.hypot(a-i,u-l)/2}}}return null}counts(){let e=0,t=0;for(let n of this.chunks.values()){e+=n.n;for(let e of n.inst)t+=e.ni}return{chunks:this.chunks.size,vertices:e,instances:t,drawnChunks:this.drawn.chunks,drawnVertices:this.drawn.vertices}}readouts(){return{chunks:()=>this.chunks.size,levelChanges:()=>this.levelChanges}}};function js(e,t){if(!e||e.length<8)return t;let n=t??[1/0,1/0,1/0,-1/0,-1/0,-1/0];for(let t=0;t+2<e.length;t+=8){let r=e[t],i=e[t+1],a=e[t+2];r<n[0]&&(n[0]=r),i<n[1]&&(n[1]=i),a<n[2]&&(n[2]=a),r>n[3]&&(n[3]=r),i>n[4]&&(n[4]=i),a>n[5]&&(n[5]=a)}return n}function Ms(e){let t=0,n=typeof e.mesh==`function`?e.far:e.mesh;if(n)for(let e=1;e<n.length;e+=8)n[e]>t&&(t=n[e]);for(let n of e.instances??[])for(let[,,e,r]of Ns(n)){let n=e+r*1.4;n>t&&(t=n)}return t}function*Ns(e){let t=e.records??e.data;if(!t)return;let n=e.records?Math.max(3,e.stride??3)*4:10;for(let e=0;e+n<=t.length;e+=n)yield[t[e],t[e+1],t[e+2],t[e+5]]}const Ps={stride:1,attributes:[{name:`r`,location:0,size:1,offset:0}]},Fs={name:`recs`,kind:`2d`,sample:`unfilterable`};function Is(e,t){let n=``,r=``;return e.forEach((e,i)=>{if(!e.formula)return;let a=zi(e.formula,{values:{value:{type:`f32`,mutable:!0},time:{type:`f32`},seed:{type:`f32`}}},{where:`${t} joint ${i}`});n+=`if (k == ${i}) {\n    var value = 0.0;\n    ${e.formula}\n    return value;\n  }\n  `,r+=`if (k == ${i}) {\n    float value = 0.0;\n    ${a.glsl.trim()}\n    return value;\n  }\n  `}),{wgsl:n,glsl:r}}function Ls(e,t,n,r){let i=Math.min(Math.floor(e.length/10),Math.floor(t.length/(n*4)));for(let a=0;a<i;a++){let i=a*10,o=a*n*4;for(let n=0;n<10;n++)t[o+n]=e[i+n];t[o+10]=(e[i+3]*.6180339887%1+1)%1,t[o+11]=r;for(let e=12;e<n*4;e++)t[o+e]=n>3?-1:0}}let Rs=-1,zs=0;const Bs=.2;function Vs(e,t,n,r,i,a){let o=e=>a?e<n?0:e<r?1:2:e>n?0:e>r?1:2;if(t<0)return o(e);let s=o(a?e/i:e*i),c=o(a?e*i:e/i);return Math.min(Math.max(t,s),c)}const Hs=(e={})=>new As(e);function Us(e,t){return t?Math.hypot(Math.max(t[0]-e[0],0,e[0]-t[3]),Math.max(t[2]-e[1],0,e[1]-t[5])):0}function Ws(e){e[0]=e[1]=e[2]=1/0,e[3]=e[4]=e[5]=-1/0}function Gs(e,t){for(let n=0;n<3;n++)t[n]<e[n]&&(e[n]=t[n]),t[n+3]>e[n+3]&&(e[n+3]=t[n+3])}const Ks=/* @__PURE__ */ new WeakMap;let qs=0;function Js(e){let t=e?e.imp??e:null;if(!t)return 0;let n=Ks.get(t);return n===void 0&&Ks.set(t,n=++qs),n}function Ys(e){let t=``;for(let n=0;n<e.length;n++)n===8||n===12||n>=16&&n<144&&(n-16)%16==14||(t+=`${e[n]},`);return t}const Xs=/* @__PURE__ */ new WeakMap;function Zs(e){return e.some(e=>{if(!(e instanceof Float32Array))return!1;let t=Xs.get(e);if(t===void 0){t=!1;for(let n=0;n<e.length/6;n+=3)if(dn(e,n)===Zt.imagery){t=!0;break}Xs.set(e,t)}return t})}export{Bi as $,ie as $n,Jt as $t,Vo as A,Ve as An,Kn as At,_a as B,Ne as Bn,gn as Bt,Bo as C,nt as Cn,mr as Ct,Uo as D,Je as Dn,lr as Dt,Do as E,Xe as En,Cr as Et,to as F,Fe as Fn,En as Ft,la as G,be as Gn,$t as Gt,ma as H,Pe as Hn,bn as Ht,qa as I,Te as In,Tn as It,Ji as J,we as Jn,rn as Jt,Xi as K,Ce as Kn,Zt as Kt,Xa as L,Ee as Ln,Cn as Lt,ko as M,ze as Mn,Ln as Mt,no as N,Re as Nn,In as Nt,Fo as O,$e as On,ur as Ot,Qa as P,Ie as Pn,Fn as Pt,Ci as Q,B as Qn,cn as Qt,Ka as R,De as Rn,Sn as Rt,Ho as S,qe as Sn,pr as St,Ro as T,Ge as Tn,hr as Tt,fa as U,Ae as Un,tn as Ut,ha as V,Me as Vn,hn as Vt,ua as W,ye as Wn,en as Wt,na as X,W as Xn,sn as Xt,Zi as Y,Se as Yn,un as Yt,Gi as Z,ve as Zn,dn as Zt,Eo as _,st as _n,O as _r,Tr as _t,Hs as a,xt as an,V as ar,Yr as at,jo as b,Ze as bn,kr as bt,ms as c,At as cn,_e as cr,qr as ct,os as d,Dt as dn,ce as dr,Ur as dt,Xt as en,z as er,zi as et,Qo as f,yt as fn,le as fr,Ar as ft,qo as g,ht as gn,T as gr,Vr as gt,Xo as h,mt as hn,H as hr,Or as ht,Vs as i,Ot as in,ue as ir,li as it,Wo as j,Le as jn,Rn as jt,zo as k,He as kn,Gn as kt,ss as l,vt as ln,se as lr,Kr as lt,Zo as m,ft as mn,de as mr,Dr as mt,As as n,qt as nn,ge as nr,ai as nt,fs as o,St as on,he as or,Qr as ot,$o as p,_t as pn,me as pr,Er as pt,Hi as q,xe as qn,on as qt,Is as r,Gt as rn,U as rr,oi as rt,ps as s,Et as sn,ae as sr,Zr as st,Bs as t,Ut as tn,re as tr,ui as tt,rs as u,gt as un,oe as ur,Wr as ut,wo as v,dt as vn,k as vr,wr as vt,Po as w,Ye as wn,cr as wt,Ao as x,Qe as xn,rr as xt,To as y,tt as yn,Ir as yt,Ja as z,ke as zn,xn as zt};