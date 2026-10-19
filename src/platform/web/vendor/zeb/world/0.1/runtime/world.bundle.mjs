import{$ as e,$n as t,$t as n,A as r,An as i,Ar as a,At as o,B as s,Bn as c,Br as l,Bt as u,C as d,Cn as f,Cr as p,Ct as m,D as h,Dn as g,Dr as _,Dt as v,E as y,En as b,Er as x,Et as S,F as C,Fn as w,Fr as T,Ft as E,G as D,Gn as O,Gt as k,H as A,Hn as j,Hr as M,Ht as N,I as P,In as F,Ir as I,It as L,J as R,Jn as z,Jt as B,K as V,Kn as H,Kt as U,L as ee,Ln as W,Lr as te,Lt as ne,M as re,Mn as ie,Mr as ae,Mt as oe,N as se,Nn as ce,Nr as le,Nt as ue,O as de,On as fe,Or as pe,Ot as me,P as he,Pn as ge,Pr as _e,Pt as ve,Q as ye,Qn as be,Qt as xe,R as Se,Rn as Ce,Rr as we,Rt as Te,S as Ee,Sn as De,Sr as G,St as Oe,T as ke,Tn as Ae,Tr as je,Tt as Me,U as Ne,Un as Pe,Ut as Fe,V as Ie,Vn as Le,Vr as Re,Vt as ze,W as Be,Wn as Ve,Wt as He,X as Ue,Xn as We,Xt as Ge,Y as Ke,Yn as qe,Yt as Je,Z as Ye,Zn as Xe,Zt as Ze,_ as Qe,_n as $e,_r as et,_t as tt,a as nt,an as rt,ar as it,at,b as ot,bn as st,br as K,bt as ct,c as lt,cn as ut,cr as dt,ct as ft,d as pt,dn as mt,dr as ht,dt as gt,en as _t,er as vt,et as yt,f as bt,fn as xt,fr as St,ft as Ct,g as wt,gn as Tt,gr as Et,gt as Dt,h as Ot,hn as kt,hr as At,ht as jt,i as Mt,in as Nt,ir as Pt,it as Ft,j as It,jn as Lt,jr as q,jt as Rt,k as zt,kn as Bt,kr as Vt,kt as Ht,l as Ut,ln as Wt,lr as Gt,lt as Kt,m as qt,mn as Jt,mr as Yt,mt as Xt,n as Zt,nn as Qt,nr as $t,nt as en,o as tn,on as nn,or as rn,ot as an,p as on,pn as sn,pr as cn,pt as ln,q as un,qn as dn,qt as fn,r as pn,rn as mn,rr as hn,rt as gn,s as _n,sn as vn,sr as yn,st as bn,tn as xn,tr as Sn,tt as Cn,u as wn,un as Tn,ur as En,ut as Dn,v as On,vn as kn,vr as An,vt as jn,w as Mn,wn as Nn,wr as J,wt as Pn,x as Fn,xn as In,xr as Ln,xt as Rn,y as zn,yn as Bn,yr as Vn,yt as Hn,z as Un,zn as Wn,zr as Y,zt as Gn}from"./solids-BIyFOe5L.mjs";const Kn=/* @__PURE__ */ new Map;function qn(e,t,n){if(n)try{let t=Kn.get(n);t||(t=new Intl.DateTimeFormat(`en-US`,{timeZone:n,year:`numeric`,month:`numeric`,day:`numeric`,hour:`numeric`,minute:`numeric`,hourCycle:`h23`}),Kn.set(n,t));let r={};for(let n of t.formatToParts(e))n.type!==`literal`&&(r[n.type]=Number(n.value));let i=Date.UTC(r.year,r.month-1,r.day,r.hour,r.minute);return{year:r.year,month:r.month,day:r.day,hour:r.hour,minute:r.minute,weekday:new Date(Date.UTC(r.year,r.month-1,r.day)).getUTCDay(),zone:n,offset:Math.round((i-Math.floor(e.getTime()/6e4)*6e4)/6e4)}}catch{}let r=Math.round(t*4),i=new Date(e.getTime()+r*6e4);return{year:i.getUTCFullYear(),month:i.getUTCMonth()+1,day:i.getUTCDate(),hour:i.getUTCHours(),minute:i.getUTCMinutes(),weekday:i.getUTCDay(),zone:`solar`,offset:r}}function Jn(e,t){let n=/* @__PURE__ */ new Date(`${e.length===16?`${e}:00`:e}Z`),r=n.getTime();for(let e=0;e<2;e++){let e=qn(new Date(r),0,t);r=n.getTime()-e.offset*6e4}return new Date(r)}let Yn=null;const Xn=Math.PI/180,Zn=()=>Yn??=import(`./zone-places-B7wsdUjy.mjs`).then(({ZONE_PLACES:e})=>e.split(`|`).map(e=>{let[t,n,r]=e.split(`,`),i=Number(n),a=Number(r);return{name:t,lon:i,lat:a,x:Math.cos(a*Xn)*Math.cos(i*Xn),y:Math.cos(a*Xn)*Math.sin(i*Xn),z:Math.sin(a*Xn)}})),Qn={nearest(e={}){let t=e.near??1500;return async(e,n)=>{let r=await Zn(),i=Math.cos(n*Xn)*Math.cos(e*Xn),a=Math.cos(n*Xn)*Math.sin(e*Xn),o=Math.sin(n*Xn),s=null,c=-2;for(let e of r){let t=e.x*i+e.y*a+e.z*o;t>c&&(c=t,s=e.name)}return Math.acos(Math.min(1,c))*6371<=t?s:null}},fixed:e=>()=>e};var $n=class{zone=null;at=null;busy=!1;ask;onChange;every;constructor(e,t,n=25){this.onChange=t,this.every=n,this.ask=typeof e==`function`?e:e===`auto`?Qn.nearest():null,typeof e==`string`&&e!==`auto`&&(this.zone=e)}zoneAt(e,t){return!this.ask||this.busy||this.at&&Math.hypot((e-this.at[0])*Math.cos(t*Xn),t-this.at[1])*111.2<this.every||(this.at=[e,t],this.busy=!0,Promise.resolve().then(()=>this.ask(e,t)).then(e=>{this.busy=!1,e!==this.zone&&(this.zone=e,this.onChange())},()=>{this.busy=!1})),this.zone}};function er(e,t,n,r){let i=((e.getUTCHours()+e.getUTCMinutes()/60+e.getUTCSeconds()/3600+t/15)%24+24)%24,a=e.getUTCMonth(),o=[`winter`,`winter`,`spring`,`spring`,`spring`,`summer`,`summer`,`summer`,`autumn`,`autumn`,`autumn`,`winter`],s={winter:`summer`,summer:`winter`,spring:`autumn`,autumn:`spring`},c=o[a];return{now:e,year:e.getUTCFullYear(),month:a+1,day:e.getUTCDate(),hour:i,season:n>=0?c:s[c],t:r,rate:1,local:qn(e,t)}}const tr=50*Math.PI/180,nr=(e,t,n=800,r=tr)=>Math.log2(n/(2*e*Math.tan(r/2))*2*Math.PI*J*Math.cos(t*G)/256),rr=(e,t,n=800,r=tr)=>n/(256*2**e/(2*Math.PI*J*Math.cos(t*G)))/(2*Math.tan(r/2));function ir(e={}){let t=e.view?.lat??0,n=e.view?.dist??(e.view?.zoom===void 0?19134411:rr(e.view.zoom,t)),r=e.view?.height??800,i={lon:0,lat:t,zoom:nr(n,t),dist:n,tilt:0,bearing:0,altitude:n,pixelSize:2*n*Math.tan(tr/2)/r,width:e.view?.width??1200,height:r,morph:0,...e.view},a=e.time?.now??new Date(Date.UTC(2026,0,1,5,0,0));return{observation:e.observation??et,view:i,time:{...er(a,i.lon,i.lat,0),...e.time},world:{params:e.params??{},readouts:e.readouts??{}},user:{selection:[],hover:null,toggles:{},role:`viewer`,...e.user},device:{api:`none`,tier:`mid`,reason:`headless`,...e.device},render:{quality:e.quality??`realistic`,detail:e.detail??1}}}const ar=88,or=Object.freeze({low:88,high:80,street:30,above:300}),sr=2*G,cr=Math.PI/2-1e-4;function lr(e,t=or){if(!(e>t.street))return t.low;if(e>=t.above)return t.high;let n=Math.log(e/t.street)/Math.log(t.above/Math.max(.001,t.street)),r=n*n*(3-2*n);return t.low+(t.high-t.low)*r}function ur(e,t=or){let n=(90-Math.max(t.low,t.high))*G,r=(90-Math.min(t.low,t.high))*G,i=n=>n>=(90-lr(e*Math.sin(n),t))*G-1e-9;if(i(n))return n;let a=n,o=r;for(let e=0;e<30;e++){let e=(a+o)/2;i(e)?o=e:a=e}return o}const dr={lon:`degrees east`,lat:`degrees north (-90..90)`,dist:`metres from the camera to the centre`,zoom:`a web-map zoom level`,tilt:`degrees from straight down (0..88; 80 at most once the eye is some 300 m up)`,bearing:`degrees clockwise from north`},fr=e=>typeof e==`number`&&Number.isFinite(e);function pr(e,t=`view`){if(!e||typeof e!=`object`||e.lon===void 0||e.lat===void 0)throw new K(`E-VIEW-001`,`a view needs numbers lon and lat (got ${JSON.stringify(e)})`,{where:t,fix:`write { lon: 107.6, lat: -6.9, zoom: 11 }`});for(let n of[`lon`,`lat`,`dist`,`zoom`,`tilt`,`bearing`]){let r=e[n];if(!(r===void 0||fr(r)))throw new K(`E-VIEW-001`,`a view's ${n} is not a finite number (got ${typeof r==`number`?String(r):JSON.stringify(r)})`,{where:t,fix:`give ${n} in ${dr[n]}${n===`lon`||n===`lat`?``:`, or leave it out`}; the view now, in the same terms, is w.camera.view (the camera's own yaw and pitch are radians)`})}if(e.dist!==void 0&&e.dist<=0)throw new K(`E-VIEW-001`,`a view's dist is ${e.dist}`,{where:t,fix:`give dist in metres above 0, or a zoom`});if(e.lat<-90||e.lat>90)throw new K(`E-VIEW-001`,`lat ${e.lat} is outside -90..90`,{where:t,fix:`did you swap lon and lat? A view is { lon, lat }`});if(e.dist!==void 0&&e.zoom!==void 0&&Math.abs(nr(e.dist,e.lat)-e.zoom)>1e-6)throw new K(`E-VIEW-001`,`a view's dist (${e.dist} m) and zoom (${e.zoom}) disagree: that dist is zoom ${+nr(e.dist,e.lat).toFixed(4)} at lat ${e.lat}`,{where:t,fix:`keep one: dist (metres) or zoom (web-map level); to change one of a view read from w.camera.view, leave the other out`})}function mr(e,t=`pose`){for(let n of[`lon`,`lat`,`dist`,`bearing`,`pitch`]){let r=e?.[n];if(!(fr(r)&&(n!==`dist`||r>0)))throw new K(`E-VIEW-001`,`a camera pose's ${n} is ${typeof r==`number`?String(r):JSON.stringify(r)}`,{where:t,fix:`a pose is { lon, lat (degrees), dist (metres above 0), bearing (degrees), pitch (radians over the horizon) }, all finite; poseOf(view) makes one from a view`})}}function hr(e){pr(e);let t=e.dist??rr(e.zoom??3,e.lat),n=Math.max(sr,Math.min(cr,(90-(e.tilt??0))*G));return{lon:e.lon,lat:e.lat,dist:t,bearing:e.bearing??0,pitch:n}}function gr(e){return{lon:e.lon,lat:e.lat,dist:e.dist,zoom:nr(e.dist,e.lat),tilt:90-e.pitch/G,bearing:e.bearing}}const _r=(e,t)=>Math.hypot(Y(t.lon-e.lon)*Math.cos((e.lat+t.lat)/2*G),t.lat-e.lat)*111320;function vr(e,t,n,r=`arc`,i=1){let a=Math.log(e.dist),o=Math.log(t.dist),s=a+(o-a)*n;if(r===`arc`){let r=Math.max(e.dist,t.dist,_r(e,t)*1.2);s+=Math.sin(Math.PI*n)*Math.max(0,Math.log(r)-Math.max(a,o))}let c=(t.bearing-e.bearing+540)%360-180,l=e.bearing+c*n+(r===`around`?360*i*n:0);return{lon:Y(e.lon+Y(t.lon-e.lon)*n),lat:e.lat+(t.lat-e.lat)*n,dist:Math.exp(s),bearing:l,pitch:e.pitch+(t.pitch-e.pitch)*n}}function yr(e,t){return .2+Math.abs(Math.log(t.dist/e.dist))+Math.log(1+_r(e,t)/Math.min(e.dist,t.dist))+Math.abs(t.pitch-e.pitch)+Math.abs((t.bearing-e.bearing+540)%360-180)/90}var br=class{poses;cum;constructor(e){if(!e.length)throw new K(`E-VIEW-001`,`a flight needs at least one view`,{fix:`pass [from, to]`});this.poses=e.map((e,t)=>`pitch`in e?(mr(e,`flight[${t}]`),e):hr(e)),this.cum=[0];for(let e=1;e<this.poses.length;e++)this.cum.push(this.cum[e-1]+yr(this.poses[e-1],this.poses[e]))}get seconds(){return Math.min(9,.8+this.cum[this.cum.length-1]*.35)}at(e){let t=this.poses;if(t.length===1||e<=0)return t[0];if(e>=1)return t[t.length-1];let n=e*this.cum[this.cum.length-1],r=1;for(;r<t.length-1&&this.cum[r]<n;)r++;let i=this.cum[r-1],a=this.cum[r];return vr(t[r-1],t[r],a>i?(n-i)/(a-i):1)}};const xr=[`direct`,`arc`,`around`];function Sr(e){if(!e.length)throw new K(`E-VIEW-001`,`camera keys: the list is empty`,{fix:`give at least one { at, view }`});e.forEach((t,n)=>{if(typeof t.at!=`number`||!Number.isFinite(t.at))throw new K(`E-VIEW-001`,`camera key ${n}: 'at' must be seconds (a number)`,{where:`keys[${n}]`});if(n&&t.at<e[n-1].at)throw new K(`E-VIEW-001`,`camera key ${n} (at ${t.at}) comes before key ${n-1} (at ${e[n-1].at})`,{where:`keys[${n}]`,fix:`sort the keys by time`});if(t.path&&!xr.includes(t.path))throw new K(`E-VIEW-001`,`camera key ${n}: unknown path '${t.path}'`,{where:`keys[${n}]`,fix:`use ${xr.join(`, `)}`});if(t.turns!==void 0&&!(typeof t.turns==`number`&&Number.isFinite(t.turns)))throw new K(`E-VIEW-001`,`camera key ${n}: turns is ${t.turns}`,{where:`keys[${n}]`,fix:`give whole turns round the centre as a finite number (1, -2), or leave it out`});En(t.ease,.5)});let t=e.map((e,t)=>{try{return hr(e.view)}catch(e){throw new K(`E-VIEW-001`,`camera key ${t}: ${e.message}`,{where:`keys[${t}]`,cause:e})}}),n=n=>{if(n<=e[0].at)return t[0];let r=e.length-1;if(n>=e[r].at)return t[r];let i=1;for(;e[i].at<n;)i++;let a=e[i-1],o=e[i],s=o.at>a.at?(n-a.at)/(o.at-a.at):1;return vr(t[i-1],t[i],En(o.ease,s),o.path??`arc`,o.turns??1)};return{list:e,duration:e[e.length-1].at-e[0].at,pose:n,at:e=>gr(n(e))}}function Cr(e,t,...n){if(!(t&&n.every(Number.isFinite)))throw new K(`E-VIEW-001`,`camera.${e}(${n.join(`, `)}): not a finite move`,{where:`camera.${e}`,fix:e===`zoomBy`?`give a factor above 0 (2: twice as far)`:`give finite numbers (metres, radians)`})}var wr=class{lon=0;lat=0;dist=J*3;yaw=0;pitch=Math.PI/2-1e-4;fov=50*Math.PI/180;morph=0;globeAbove=32e5;flatBelow=11e5;flat=!1;projection=`auto`;morphEase=0;touches=0;hands={pan:0,turn:0,zoom:0};drag=`pan`;held=!1;interrupt=!0;minDist=40;maxDist=J*8;gestures={pan:!0,rotate:!0,tilt:!0,zoom:!0};pitchRange=[2*G,Math.PI/2-1e-4];tiltLimit={...or};pitchFloor(e=this.dist){return ur(e,this.tiltLimit)}pitchAt(e){return(90-lr(e,this.tiltLimit))*G}zoomLimited=!1;groundY=0;viewProj=l.create();eye=Re.create();inv=l.create();width=1200;height=800;clock=0;motion=null;playing=null;driver=null;zoomLeft=1;zoomAtPx=[0,0];glide=[0,0];constructor(e){e&&this.jump(e)}changed=null;get pose(){return{lon:this.lon,lat:this.lat,dist:this.dist,bearing:Y(-this.yaw/G),pitch:this.pitch}}get view(){return gr({...this.pose,dist:this.dist/this.frameK})}get zoom(){return nr(this.dist/this.frameK,this.lat)}frame=`auto`;get frameK(){return this.frame!==`auto`||this.width>=this.height?1:Math.min(2.5,Math.max(1,1.2*this.height/Math.max(1,this.width)))}get pixelSize(){return 2*this.dist*Math.tan(this.fov/2)/Math.max(1,this.height)}jump(e){let t=`pitch`in e?e:this.framed(hr(e));mr(t,`jump`),this.stop(),this.setPose(t),this.changed?.()}setPose(e,t=`pose`){mr(e,t),this.lon=Y(e.lon),this.lat=Math.max(-p,Math.min(p,e.lat)),this.dist=Math.min(this.maxDist,Math.max(this.minDist,e.dist)),this.yaw=-e.bearing*G,this.pitch=e.pitch,this.limitTilt()}limitTilt(){this.pitch<(90-Math.min(this.tiltLimit.low,this.tiltLimit.high))*G&&(this.pitch=Math.max(this.pitch,this.pitchFloor()))}place(e,t){this.setPose(e,`move`),this.dist=Math.min(J*8,Math.max(.01,e.dist)),this.limitTilt(),t!==void 0&&Number.isFinite(t)&&(this.groundY=t)}framed(e){let t=this.frameK;return t===1?e:{...e,dist:e.dist*t}}fly(e,t={}){let n=this.framed(hr(e));if(t.seconds!==void 0&&!(Number.isFinite(t.seconds)&&t.seconds>=0))throw new K(`E-VIEW-001`,`a flight's seconds is ${t.seconds}`,{where:`fly`,fix:`give seconds as a finite number, 0 or more (0: at once), or leave it out`});if(t.path!==void 0&&!xr.includes(t.path))throw new K(`E-VIEW-001`,`unknown path '${t.path}'`,{where:`fly`,fix:`use ${xr.join(`, `)}`});ht(t.ease),this.stop();let r=this.pose;e.bearing===void 0&&(n.bearing=r.bearing),e.tilt===void 0&&(n.pitch=r.pitch);let i=Math.hypot(Y(n.lon-r.lon),n.lat-r.lat)*111320,a=t.seconds??Math.min(6,.9+.35*(Math.abs(Math.log(n.dist/r.dist))+Math.log(1+i/Math.min(r.dist,n.dist))));return new Promise(e=>{this.motion={from:r,to:n,t0:NaN,seconds:a,ease:t.ease??`smooth`,path:t.path??`arc`,done:e},this.changed?.()})}play(e,t={}){return this.stop(),new Promise(n=>{this.playing={keys:e,t0:NaN,loop:!!t.loop,done:n},this.changed?.()})}get moving(){return!!(this.motion||this.playing||this.driver||this.zoomLeft!==1||this.glide[0]||this.glide[1])||this.changedFrames>0}last=[NaN,NaN,NaN,NaN,NaN];changedFrames=0;noteChange(){let e=this.last,t=(e,t)=>e!==t&&!(Number.isNaN(e)&&Number.isNaN(t));t(e[0],this.lon)||t(e[1],this.lat)||t(e[2],this.dist)||t(e[3],this.yaw)||t(e[4],this.pitch)?(this.changedFrames=Number.isNaN(e[0])?0:2,e[0]=this.lon,e[1]=this.lat,e[2]=this.dist,e[3]=this.yaw,e[4]=this.pitch):this.changedFrames>0&&this.changedFrames--}stop(){let e=this.motion,t=this.playing,n=this.driver;this.motion=null,this.playing=null,this.driver=null,this.zoomLeft=1,this.glide=[0,0],e?.done(),t?.done(),n?.done()}drive(e){return this.stop(),new Promise(t=>{this.driver={step:e,done:t},this.changed?.()})}get driving(){return this.driver!==null}byHand(e){if(this.hands[e]++,this.driver&&this.interrupt){let e=this.driver;this.driver=null,e.done()}}update(e,t,n){let r=this.frameK;this.width=e,this.height=t;let i=this.frameK;r!==i&&this.zoomLimited&&(this.minDist*=i/r,this.maxDist*=i/r),r!==i&&!this.motion&&!this.playing&&(this.dist=Math.min(this.maxDist,Math.max(this.minDist,this.dist*(i/r))));let a=Math.min(64,Math.max(0,n));if(this.clock+=n/1e3,this.zoomLeft!==1){let e=Math.abs(Math.log(this.zoomLeft))<1e-4?this.zoomLeft:this.zoomLeft**(1-Math.exp(-a/90));this.zoomByHand(e,...this.zoomAtPx),this.zoomLeft/=e,Math.abs(Math.log(this.zoomLeft))<1e-4&&(this.zoomLeft=1)}if(this.glide[0]||this.glide[1]){this.panByHand(this.glide[0]*a,this.glide[1]*a);let e=Math.exp(-a/170);this.glide=[this.glide[0]*e,this.glide[1]*e],Math.hypot(...this.glide)<.01&&(this.glide=[0,0])}if(this.motion){let e=this.motion;Number.isNaN(e.t0)&&(e.t0=this.clock);let t=e.seconds>0?Math.min(1,(this.clock-e.t0)/e.seconds):1;try{this.setPose(vr(e.from,e.to,En(e.ease,t),e.path),`fly`)}catch(e){throw this.stop(),e}t>=1&&(this.motion=null,e.done())}if(this.playing){let e=this.playing,t=e.keys.duration;Number.isNaN(e.t0)&&(e.t0=this.clock-e.keys.list[0].at);let n=this.clock-e.t0;e.loop&&t>0&&(n=e.keys.list[0].at+(n-e.keys.list[0].at)%t);try{this.setPose(this.framed(e.keys.pose(n)),`play`)}catch(e){throw this.stop(),e}!e.loop&&n>=e.keys.list[e.keys.list.length-1].at&&(this.playing=null,e.done())}if(this.driver){let e=this.driver,t=!1;try{t=e.step(n/1e3)}catch(t){throw this.driver===e&&(this.driver=null,e.done()),t}t&&this.driver===e&&(this.driver=null,e.done())}this.limitTilt();let o=this.morphAt(this.dist);this.morphEase>0?(this.morph+=(o-this.morph)*Math.min(1,a/1e3/this.morphEase*4),Math.abs(o-this.morph)<.002&&(this.morph=o,this.morphEase=0)):this.morph=o,this.keepClear(),this.noteChange(),this.matrices()}setProjection(e,t=1.2){this.projection=e,this.morphEase=Math.max(0,t),this.changed?.()}morphAt(e){if(this.flat||this.projection===`flat`)return 1;if(this.projection===`globe`)return 0;let t=Math.min(1,Math.max(0,(e-this.flatBelow)/(this.globeAbove-this.flatBelow)));return 1-t*t*(3-2*t)}clearance=0;clear=1;under=null;clearPitch=0;keepClear(){this.clearPitch=0;let e=this.under,t=this.clear;if(!e||t===!1||this.morph<.5)return;let n=this.dist*Math.sin(this.pitch);if(n>3e3)return;let r=this.dist*Math.cos(this.pitch),i=-this.yaw,a=J*G,o=a*Math.cos(this.lat*G),s=Y(this.lon-Math.sin(i)*r/o),c=this.lat-Math.cos(i)*r/a;if(!Number.isFinite(s)||!Number.isFinite(c)||!Number.isFinite(n)||!Number.isFinite(this.groundY))return;let l=e(s,c,this.groundY+n);if(!Number.isFinite(l))return;let u=l+t-this.groundY;n>=u||(this.clearPitch=u>=this.dist?Math.PI/2-1e-4:Math.asin(u/this.dist))}planes(e=this.dist){return[Math.max(.05,e*.004,Math.min(this.clearance*.1,e*.02)),e*40+1e3]}matrices(){let e=Math.max(this.pitch,this.clearPitch),t=Math.cos(e),n=[0,this.groundY,0];Re.set(this.dist*t*Math.sin(this.yaw),this.groundY+this.dist*Math.sin(e),this.dist*t*Math.cos(this.yaw),this.eye);let r=e>1.5?[Math.sin(this.yaw)*-1,0,-Math.cos(this.yaw)]:[0,1,0],[i,a]=this.planes();l.multiply(l.perspective(this.fov,this.width/Math.max(1,this.height),i,a),l.lookAt(this.eye,n,r),this.viewProj),l.inverse(this.viewProj,this.inv)}ray(e,t){let n=e/this.width*2-1,r=1-t/this.height*2,i=M.transformMat4([n,r,0,1],this.inv),a=M.transformMat4([n,r,1,1],this.inv),o=[i[0]/i[3],i[1]/i[3],i[2]/i[3]],s=[a[0]/a[3],a[1]/a[3],a[2]/a[3]],c=Math.hypot(s[0]-o[0],s[1]-o[1],s[2]-o[2])||1;return{o,d:[(s[0]-o[0])/c,(s[1]-o[1])/c,(s[2]-o[2])/c]}}groundHit(e,t,n=this.groundY){let{o:r,d:i}=this.ray(e,t);if(this.morph<.5){let e=[0,-J,0],t=J+n,a=[r[0]-e[0],r[1]-e[1],r[2]-e[2]],o=a[0]*i[0]+a[1]*i[1]+a[2]*i[2],s=a[0]**2+a[1]**2+a[2]**2-t*t,c=o*o-s;if(c<0)return null;let l=-o-Math.sqrt(c);return l>0?[r[0]+i[0]*l,r[1]+i[1]*l,r[2]+i[2]*l]:null}if(Math.abs(i[1])<1e-9)return null;let a=(n-r[1])/i[1];return a>0?[r[0]+i[0]*a,n,r[2]+i[2]*a]:null}lonLatAt(e,t){let n=this.groundHit(e,t);return n?a(this,n[0],n[1],n[2]):null}projectWorld(e,t,n){let r=M.transformMat4([e,t,n,1],this.viewProj);return r[3]<=0?null:[(r[0]/r[3]+1)/2*this.width,(1-r[1]/r[3])/2*this.height]}project(e,t,n=0){let r=we(this,e,t,n);return this.projectWorld(r[0],r[1],r[2])}facing(e,t,n=0){if(this.morph>.98)return!0;let r=we(this,e,t,n),i=[r[0],r[1]+J,r[2]],a=this.eye;return i[0]*(a[0]-r[0])+i[1]*(a[1]-r[1])+i[2]*(a[2]-r[2])>0}zoomBy(e){Cr(`zoomBy`,e>0,e),this.dist=Math.min(this.maxDist,Math.max(this.minDist,this.dist*e))}zoomAt(e,t,n){if(this.morph<.6||this.held){this.zoomBy(e);return}let r=this.groundHit(t,n),i=this.dist;this.zoomBy(e);let a=this.dist/i;!r||Math.hypot(r[0],r[2])>i*20||this.moveBy(r[0]*(1-a),r[2]*(1-a))}pan(e,t){let n=2*this.dist*Math.tan(this.fov/2)/this.height,r=Math.cos(this.yaw),i=Math.sin(this.yaw),a=1/Math.max(.35,Math.sin(this.pitch));this.moveBy((-r*e-i*t*a)*n,(i*e-r*t*a)*n)}moveBy(e,t){Cr(`moveBy`,!0,e,t),this.lon=Y(this.lon+e/(J*Math.max(.05,Math.cos(this.lat*G)))/G),this.lat=Math.max(-p,Math.min(p,this.lat-t/J/G))}turn(e,t){Cr(`turn`,!0,e,t),this.yaw+=e,this.pitch=Math.min(this.pitchRange[1],Math.max(this.pitchRange[0],this.pitch+t)),this.limitTilt()}turnByHand(e,t){let n=this.gestures.rotate?e:0,r=this.gestures.tilt?t:0;(n||r)&&(this.turn(n,r),this.byHand(`turn`))}panByHand(e,t){(e||t)&&(this.pan(e,t),this.byHand(`pan`))}zoomByHand(e,t,n){e!==1&&(this.zoomAt(e,t,n),this.byHand(`zoom`))}attach(e,t){let n=/* @__PURE__ */ new Map,r=0,i=0,a=0,o=null,s=0,c=[0,0],l=0,u=(t,n)=>{let r=e.getBoundingClientRect();return[t-r.left,n-r.top]},d=()=>{let[e,t]=[...n.values()];return{d:Math.hypot(e.x-t.x,e.y-t.y),ang:Math.atan2(t.y-e.y,t.x-e.x),m:[(e.x+t.x)/2,(e.y+t.y)/2]}},f=e.style.touchAction;e.style.touchAction=`none`;let p=[],m=(t,n,r)=>{e.addEventListener(t,n,r),p.push([t,n,r])};m(`contextmenu`,e=>e.preventDefault()),m(`pointerdown`,t=>{if(!this.interrupt&&(this.motion||this.playing||this.driver))return;let l=this.driver;if(this.driver=null,e.setPointerCapture?.(t.pointerId),n.set(t.pointerId,{x:t.clientX,y:t.clientY}),this.stop(),this.driver=l,this.touches++,n.size===1&&(r=0,c=[0,0]),n.size===2){let e=d();i=e.d,a=e.ang,o=e.m,s=e.m[1]}}),m(`pointermove`,e=>{let t=n.get(e.pointerId);if(!t)return;this.changed?.();let f=e.clientX-t.x,p=e.clientY-t.y;if(t.x=e.clientX,t.y=e.clientY,r+=Math.abs(f)+Math.abs(p),n.size===1){if((!(e.buttons&2)&&!e.shiftKey)==(this.drag===`pan`)){if(!this.gestures.pan)return;this.panByHand(f,p);let e=performance.now(),t=Math.max(1,e-l);l=e,c=[c[0]*.6+f/t*.4,c[1]*.6+p/t*.4]}else this.turnByHand(-f*.006,p*.005)}else if(n.size===2&&o){let e=d(),t=this.gestures;t.zoom&&i>0&&this.zoomByHand(i/Math.max(1,e.d),...u(e.m[0],e.m[1])),t.rotate&&e.ang!==a&&(this.yaw+=e.ang-a,this.byHand(`turn`));let n=e.m[1]-s;Math.abs(e.d-i)<4&&this.turnByHand(0,n*.004),t.pan&&this.panByHand(e.m[0]-o[0],0),c=[0,0],i=e.d,a=e.ang,o=e.m,s=e.m[1]}});let h=i=>{if(n.size===1&&r<8&&i.type===`pointerup`){let n=e.getBoundingClientRect();t?.(i.clientX-n.left,i.clientY-n.top)}else n.size===1&&performance.now()-l<60&&Math.hypot(...c)>.15&&(this.glide=[c[0],c[1]]);n.delete(i.pointerId),n.size<2&&(o=null)};return m(`pointerup`,h),m(`pointercancel`,h),m(`wheel`,e=>{if(e.preventDefault(),!this.gestures.zoom||!this.interrupt&&(this.motion||this.playing||this.driver))return;if(this.motion||this.playing){let e=this.driver;this.driver=null,this.stop(),this.driver=e}this.glide=[0,0],this.touches++;let t=e.deltaY*(e.deltaMode===1?16:1)*(e.ctrlKey?.01:.0018);this.zoomLeft=Math.min(8,Math.max(1/8,this.zoomLeft*Math.exp(t))),this.zoomAtPx=u(e.clientX,e.clientY),this.changed?.()},{passive:!1}),()=>{for(let[t,n,r]of p)e.removeEventListener(t,n,r);e.style.touchAction=f}}};const Tr=1.6783469900166608,Er=Object.freeze({place:Object.freeze({halfLife:.35}),aim:Object.freeze({halfLife:.35}),dist:Object.freeze({halfLife:.4}),heading:Object.freeze({halfLife:.7,deadZone:.5,from:`motion`})}),Dr=e=>typeof e==`number`&&Number.isFinite(e);function Or(e,t=`damping`){if(e!==void 0){if(!e||typeof e!=`object`)throw new K(`E-VIEW-001`,`${t}: ${JSON.stringify(e)} is not a damping`,{where:t,fix:`give { place, aim, dist, heading } springs ({ halfLife: seconds }), or leave it out`});for(let n of[`place`,`aim`,`dist`,`heading`]){let r=e[n];if(r!==void 0)for(let e of[`halfLife`,`stiffness`,`deadZone`,`maxSpeed`,`lead`]){let i=r[e];if(i!==void 0&&!(Dr(i)&&i>=0))throw new K(`E-VIEW-001`,`${t}: ${n}.${e} is ${i}`,{where:t,fix:`give ${n}.${e} as a finite number, 0 or more, or leave it out`})}}if(e.heading?.from!==void 0&&e.heading.from!==`motion`&&e.heading.from!==`thing`)throw new K(`E-VIEW-001`,`${t}: heading.from is ${JSON.stringify(e.heading.from)}`,{where:t,fix:`give heading.from as 'motion' (the way it goes) or 'thing' (its own heading), or leave it out`});if(e.snap!==void 0&&e.snap!==!1&&!(Dr(e.snap)&&e.snap>0))throw new K(`E-VIEW-001`,`${t}: snap is ${e.snap}`,{where:t,fix:`give snap in metres above 0, false (never), or leave it out`})}}function kr(e){return e?e.stiffness===void 0?e.halfLife?Tr/e.halfLife:0:Math.sqrt(e.stiffness):0}function Ar(e,t,n,r,i,a){if(!(i>0))return[e,t];let o=(r-n)/i;if(!(a>0))return[r,o];let s=e-(n-2*o/a),c=t-o,l=Math.exp(-a*i),u=c+a*s;return[r-2*o/a+(s+u*i)*l,o+(c-a*u*i)*l]}var jr=class{x=[];v=[];goal=[];on=!1;n;eps;constructor(e,t=.001){this.n=e,this.eps=t}reset(){this.on=!1}start(e,t){this.x=[...e],this.goal=[...e],this.v=t?[...t]:e.map(()=>0),this.on=!0}step(e,t,n,r){if(!this.on)return this.start(e),this.x;let i=this.goal,a=n?.deadZone??0,o=[...e];if(a>0){let t=0;for(let n=0;n<this.n;n++)t+=(e[n]-i[n])**2;let n=Math.sqrt(t);o=n<=a?[...i]:i.map((t,r)=>t+(e[r]-t)*(n-a)/n)}let s=[...this.x];for(let e=0;e<this.n;e++)[this.x[e],this.v[e]]=Ar(this.x[e],this.v[e],i[e],o[e],t,r);let c=n?.maxSpeed;if(c!==void 0&&t>0){let e=0;for(let t=0;t<this.n;t++)e+=(this.x[t]-s[t])**2;let n=Math.sqrt(e),r=c*t;if(n>r)for(let e=0;e<this.n;e++)this.x[e]=s[e]+(this.x[e]-s[e])*r/n;let i=0;for(let e=0;e<this.n;e++)i+=this.v[e]**2;let a=Math.sqrt(i);if(a>c)for(let e=0;e<this.n;e++)this.v[e]=this.v[e]*c/a}this.goal=o;let l=0,u=0;for(let e=0;e<this.n;e++)l=Math.max(l,Math.abs(this.x[e]-o[e])),u=Math.max(u,Math.abs(this.v[e]));return l<this.eps&&u<this.eps*4&&o.every((e,t)=>e===i[t])&&(this.x=[...o],this.v=this.v.map(()=>0)),this.x}shift(e){for(let t=0;t<this.n;t++)this.x[t]=this.x[t]-e[t],this.goal[t]=this.goal[t]-e[t]}};const Mr=J*G,Nr=e=>Mr*Math.cos(e*G),Pr=(e,t)=>[Y(t[0]-e[0])*Nr(e[1]),(t[1]-e[1])*Mr,t[2]-e[2]];var Fr=class{damping;place=new jr(3);aim=new jr(3);dist=new jr(1,1e-5);heading=new jr(1,.001);origin={place:null,aim:null};vel=new jr(3,.001);trackAt=null;lastTarget={place:null,aim:null};constructor(e={}){Or(e,`damping`),this.damping=e}reset(){this.place.reset(),this.aim.reset(),this.dist.reset(),this.heading.reset(),this.vel.reset(),this.trackAt=null,this.origin={place:null,aim:null},this.lastTarget={place:null,aim:null}}seed(e){for(let t of[`place`,`aim`]){let n=e[t];n&&(this.origin[t]=n,this[t].start([0,0,0]),this.lastTarget[t]=null)}e.dist!==void 0&&this.dist.start([Math.log(Math.max(.001,e.dist))]),typeof e.heading==`number`&&this.heading.start([e.heading])}has(e){return this[e].on}get headingFrom(){return this.damping.heading?.from??Er.heading.from}spring(e){if(e===`aim`&&!this.damping.aim)return this.spring(`place`);let t=this.damping[e],n=Er[e];return t?{...n,...t.stiffness!==void 0&&t.halfLife===void 0?{halfLife:void 0}:{},...t}:n}step(e,t,n=100){let r={cut:!1,velocity:[0,0,0]},i=this.damping.snap===!1?1/0:this.damping.snap??Math.max(1,2*n),a=e.place??e.aim,o=e.place?this.lastTarget.place:this.lastTarget.aim,s=this.vel;if(!a)s.reset(),this.trackAt=null;else if(!s.on||!o||!this.trackAt||Math.hypot(...Pr(o,a))>i)s.start([0,0,0]),this.trackAt=a;else{let n=this.trackAt,r=[n[0]+s.x[0]/Nr(n[1]),n[1]+s.x[1]/Mr,n[2]+s.x[2]];s.shift(s.x.slice()),this.trackAt=r;let i=Pr(r,a);s.step(i,t,void 0,kr(this.spring(e.place?`place`:`aim`))||1.6783469900166608/.25)}let c=s.on?s.v:[0,0,0];r.velocity=[c[0],c[1],c[2]];for(let n of[`place`,`aim`]){let a=e[n];if(!a){this[n].on&&(this[n].reset(),this.origin[n]=null,this.lastTarget[n]=null);continue}let o=this.spring(n),s=this[n],l=this.lastTarget[n],u=l?Math.hypot(...Pr(l,a)):s.on&&this.origin[n]?Math.hypot(...Pr(this.origin[n],a)):0;if(!s.on||u>i){s.on&&(r.cut=!0),this.origin[n]=a,s.start([0,0,0]),this.lastTarget[n]=a,r[n]=[a[0],a[1],a[2]];continue}let d=this.origin[n],f=[d[0]+s.x[0]/Nr(d[1]),d[1]+s.x[1]/Mr,d[2]+s.x[2]];s.shift(s.x.slice()),this.origin[n]=f;let p=kr(o),m=o.lead??(p>0?2/p:0),h=Pr(f,a),g=s.step([h[0]+c[0]*m,h[1]+c[1]*m,h[2]+c[2]*m],t,o,p);r[n]=[f[0]+g[0]/Nr(f[1]),f[1]+g[1]/Mr,f[2]+g[2]],this.lastTarget[n]=a}if(e.dist!==void 0){let n=this.spring(`dist`),i=this.dist,a=Math.log(Math.max(.001,e.dist));if(!i.on||r.cut&&this.damping.snap!==!1)i.start([a]);else{let e=Math.exp(i.x[0]),r=n.deadZone,o=n.maxSpeed;i.step([a],t,{deadZone:r?r/e:0,maxSpeed:o===void 0?void 0:o/e},kr(n))}r.dist=Math.exp(i.x[0])}else this.dist.reset();let l=Math.hypot(c[0],c[1])>.5,u=e.heading===void 0?void 0:l&&(this.headingFrom===`motion`||e.heading===null)?(Math.atan2(c[0],c[1])/G+360)%360:e.heading===null?this.heading.on?this.heading.goal[0]:void 0:e.heading;if(u!==void 0){let e=this.spring(`heading`),n=this.heading;if(!n.on||r.cut?n.start([u]):n.step([n.x[0]+((u-n.x[0])%360+540)%360-180],t,e,kr(e)),Math.abs(n.x[0])>3600){let e=Math.round(n.x[0]/360)*360;n.shift([e])}r.heading=(n.x[0]%360+360)%360}else e.heading===void 0&&this.heading.reset();return r}};function Ir(e,t,n={}){if(!e||typeof e.at!=`function`)throw new K(`E-VIEW-001`,`track: not a log`,{where:`track`,fix:`track(history.log(…), id)`});return{track:e,id:t,up:n.up}}function Lr(e,t={}){Rr(e,`lookAt`);for(let e of[`lead`,`raise`]){let n=t[e];if(n!==void 0&&!(typeof n==`number`&&Number.isFinite(n)))throw new K(`E-VIEW-001`,`lookAt: ${e} is ${n}`,{where:`lookAt`,fix:`give ${e} as a finite number, or leave it out`})}return{kind:`look-at`,target:e,lead:t.lead,raise:t.raise}}function Rr(e,t){if(!(typeof e==`function`||typeof e==`string`||Array.isArray(e)&&e.length>=2&&Number.isFinite(e[0])&&Number.isFinite(e[1])||e&&typeof e==`object`&&(`track`in e&&typeof e.id==`string`||Number.isFinite(e.lon)&&Number.isFinite(e.lat))))throw new K(`E-VIEW-001`,`${t}: ${JSON.stringify(e)} is not a place or a thing`,{where:t,fix:`give [lon, lat, h?], { lon, lat, up?, heading? }, a point's name, track(log, id) or a function of the time`})}function zr(e,t,n){let r=t.exaggeration??1,i=(e,n)=>{let i=t.heightAt(e,n);return Number.isFinite(i)?i*r:0},a;if(typeof e==`function`)a=e(n);else if(typeof e==`string`)a=t.pointAt?.(e)??null;else if(!Array.isArray(e)&&e&&typeof e==`object`&&`track`in e){let t=e.track.at(e.id,n);if(!t)return null;let r=typeof e.up==`function`?e.up(t.lon,t.lat):e.up??0;a={lon:t.lon,lat:t.lat,up:r,heading:t.heading}}else a=e;if(!a)return null;if(Array.isArray(a)){let[e,t,n]=a;if(!Number.isFinite(e)||!Number.isFinite(t))return null;let o=i(e,t);return{lon:e,lat:t,y:n!==void 0&&Number.isFinite(n)?n*r:o,g:o,heading:NaN}}let o=a;if(!Number.isFinite(o.lon)||!Number.isFinite(o.lat))return null;let s=i(o.lon,o.lat);return{lon:o.lon,lat:o.lat,y:s+(Number.isFinite(o.up)?o.up:0),g:s,heading:Number.isFinite(o.heading)?o.heading:NaN}}const Br=J*G,Vr=e=>Br*Math.cos(e*G),Hr=(e,t,n,r)=>[Y(e+Math.sin(n*G)*r/Vr(t)),t+Math.cos(n*G)*r/Br];function Ur(e,t,n,r,i=()=>0){let a=(90-Math.max(0,Math.min(89.99,t.tilt)))*G,o=Math.max(.01,t.dist),s=e.y+o*Math.sin(a),c=-o*Math.cos(a),l=Math.atan((t.raise??0)*Math.tan(r/2)),u=Math.min(Math.PI/2-1e-4,a+l),d=n(e.lon,e.lat),f=0,p=[e.lon,e.lat],m=Math.max(i(s-d),u);for(let r=0;r<3;r++){m=Math.max(i(s-d),u),f=Math.max(.01,(s-d)/Math.sin(m)),p=Hr(e.lon,e.lat,t.bearing,c+f*Math.cos(m));let r=n(p[0],p[1]);if(Math.abs(r-d)<.01){d=r;break}d=r}return f=Math.max(.01,(s-d)/Math.sin(m)),{pose:{lon:p[0],lat:p[1],dist:f,bearing:(t.bearing%360+360)%360,pitch:m},ground:d}}function Wr(e,t,n){let r=Vr(t.lat),i=Y(t.lon-e.lon)*r,a=(t.lat-e.lat)*Br,o=t.y-e.y,s=Math.hypot(i,a);return{dist:Math.max(.01,Math.hypot(s,o)),tilt:90-Math.atan2(-o,s)/G,bearing:s>1e-6?(Math.atan2(i,a)/G+360)%360:0,raise:n}}function Gr(e,t){let n=(90-Math.max(0,Math.min(89.99,t.tilt)))*G,r=Hr(e.lon,e.lat,t.bearing,-t.dist*Math.cos(n));return{lon:r[0],lat:r[1],y:e.y+t.dist*Math.sin(n)}}function Kr(e,t){let n=Hr(e.lon,e.lat,e.bearing,-e.dist*Math.cos(e.pitch));return{lon:n[0],lat:n[1],y:t+e.dist*Math.sin(e.pitch)}}const qr=e=>typeof e==`number`&&Number.isFinite(e),Jr=[`dist`,`tilt`,`bearing`,`turn`,`raise`];function Yr(e){let t=`moves.${e.kind??`move`}`,n=Xr(e.aim)?e.aim:Lr(e.aim);if(!Array.isArray(e.keys)||!e.keys.length)throw new K(`E-VIEW-001`,`${t}: a move needs keys`,{where:t,fix:`give keys: [{ at: 0, rig: { dist, tilt, bearing } }, …] (rig 'now': where the camera is)`});if(e.keys.forEach((n,r)=>{if(!qr(n.at)||n.at<0||n.at>1)throw new K(`E-VIEW-001`,`${t}: key ${r}'s at is ${n.at}`,{where:t,fix:`give at as a share of the move, 0..1`});if(r&&n.at<e.keys[r-1].at)throw new K(`E-VIEW-001`,`${t}: key ${r} comes before key ${r-1}`,{where:t,fix:`sort the keys by at`});if(n.rig!==`now`)for(let e of Jr){let i=n.rig?.[e];if(i!==void 0&&!qr(i))throw new K(`E-VIEW-001`,`${t}: key ${r}'s ${e} is ${i}`,{where:t,fix:`give ${e} as a finite number, or leave it out`})}if(n.rig!==`now`&&n.rig?.dist!==void 0&&!(n.rig.dist>0))throw new K(`E-VIEW-001`,`${t}: key ${r}'s dist is ${n.rig.dist}`,{where:t,fix:`give dist in metres above 0`});ht(n.ease)}),Or(e.damping,t),e.over!==void 0&&e.over!==!1&&!(qr(e.over)&&e.over>=0))throw new K(`E-VIEW-001`,`${t}: over is ${e.over}`,{where:t,fix:`give over in metres, 0 or more, false, or leave it out`});if(e.seconds!==void 0&&!(qr(e.seconds)&&e.seconds>=0))throw new K(`E-VIEW-001`,`${t}: seconds is ${e.seconds}`,{where:t,fix:`give seconds as a finite number, 0 or more, or leave it out`});return{kind:e.kind??`move`,aim:n,keys:e.keys,damping:e.damping,over:e.over,seconds:e.seconds}}const Xr=e=>!!e&&typeof e==`object`&&e.kind===`look-at`,Zr=(e,t)=>Xr(e)?e:(Rr(e,t),Lr(e)),Qr=(e,t)=>({dist:Math.max(.01,Math.hypot(e,t)),tilt:90-Math.atan2(t,e)/G}),$r={chase(e,t={}){let n={...Qr(t.behind??12,t.up??4),turn:t.turn??0,raise:t.raise};return Yr({kind:`chase`,aim:Zr(e,`moves.chase`),keys:t.to?[{at:0,rig:n},{at:1,rig:t.to,ease:t.ease??`smooth`}]:[{at:0,rig:n}],damping:t.damping,over:t.over??4,seconds:t.seconds})},track(e,t={}){let n=t.side??8,r={...Qr(Math.abs(n),t.up??2),turn:n>=0?90:-90,raise:t.raise};return Yr({kind:`track`,aim:Zr(e,`moves.track`),keys:t.to?[{at:0,rig:r},{at:1,rig:t.to,ease:t.ease??`smooth`}]:[{at:0,rig:r}],damping:t.damping,over:t.over??!1,seconds:t.seconds})},crane(e,t={}){return Yr({kind:`crane`,aim:Zr(e,`moves.crane`),keys:[{at:0,rig:`now`},{at:1,rig:t.to??{crane:[t.up??30,t.back??0,t.swing??0]},ease:t.ease??`smooth`}],damping:t.damping,over:t.over??!1,seconds:t.seconds})},dolly(e,t={}){if(t.to!==void 0&&!(qr(t.to)&&t.to>0))throw new K(`E-VIEW-001`,`moves.dolly: to is ${t.to}`,{where:`moves.dolly`,fix:`give to as a distance in metres above 0, or in / out`});return Yr({kind:`dolly`,aim:Zr(e,`moves.dolly`),keys:[{at:0,rig:`now`},{at:1,rig:t.to===void 0?{dolly:(t.out??0)-(t.in??0)}:{dist:t.to},ease:t.ease??`smooth`}],damping:t.damping,over:t.over??!1,seconds:t.seconds})},arc(e,t={}){let n={at:0,rig:t.from??`now`};return Yr({kind:`arc`,aim:Zr(e,`moves.arc`),keys:[n,{at:1,rig:{arc:t.degrees??90,dist:t.dist,tilt:t.tilt},ease:t.ease??`smooth`}],damping:t.damping,over:t.over??!1,seconds:t.seconds})},reveal(e,t={}){let n={tilt:55,...t.to,dist:t.to?.dist??t.dist??40},r=n.dist,i=t.from??`behind`,a=i===`below`?{dist:r/4,tilt:84,turn:n.turn,bearing:n.bearing}:i===`side`?{dist:r/4,tilt:(n.tilt??55)+15,turn:(n.turn??0)+90}:{dist:r/4,tilt:Math.min(80,(n.tilt??55)+20),turn:n.turn,bearing:n.bearing};return Yr({kind:`reveal`,aim:Zr(e,`moves.reveal`),keys:[{at:0,rig:{...a,raise:t.raise}},{at:1,rig:n,ease:t.ease??`cinematic`}],damping:t.damping,over:t.over??!1,seconds:t.seconds})}},ei=(e,t)=>((t-e)%360+540)%360-180;var ti=class{move;seconds;follower;t=0;keys=null;lift=0;liftV=0;lifted=!1;ease;constructor(e,t={},n){if(t.seconds!==void 0&&!(qr(t.seconds)&&t.seconds>=0))throw new K(`E-VIEW-001`,`play: seconds is ${t.seconds}`,{where:`moves.${e.kind}`,fix:`give seconds as a finite number, 0 or more, or leave it out`});ht(t.ease),this.move=e,this.seconds=t.seconds??e.seconds,this.ease=t.ease,this.follower=n??new Fr(e.damping),n&&e.damping&&(n.damping=e.damping)}progress(e=this.t){let t=this.seconds;return t===void 0?0:t>0?Math.min(1,e/t):1}get done(){return this.seconds!==void 0&&this.t>=this.seconds}resolve(e,t,n){let r=[],i=Wr(Kr(e.pose,e.groundY),t),a={dist:i.dist,tilt:i.tilt,raise:0,...Number.isFinite(n)?{turn:ei(n,i.bearing)}:{bearing:i.bearing}};for(let e of this.move.keys){if(e.rig===`now`){r.push(a={...a});continue}let t=e.rig,n={...a};if(t.crane){let e=(90-a.tilt)*G,r=a.dist*Math.sin(e)+t.crane[0],i=Math.max(.01,a.dist*Math.cos(e)+t.crane[1]);n.dist=Math.hypot(r,i),n.tilt=90-Math.atan2(r,i)/G,n.turn===void 0?n.bearing=(n.bearing??0)+t.crane[2]:n.turn+=t.crane[2]}t.dolly!==void 0&&(n.dist=Math.max(.5,a.dist+t.dolly)),t.arc!==void 0&&(n.turn===void 0?n.bearing=(n.bearing??0)+t.arc:n.turn+=t.arc),t.dist!==void 0&&(n.dist=t.dist),t.tilt!==void 0&&(n.tilt=t.tilt),t.raise!==void 0&&(n.raise=t.raise),t.bearing===void 0?t.turn!==void 0&&(n.turn=t.turn,delete n.bearing):(n.bearing=t.bearing,delete n.turn),r.push(a=n)}return r}rigAt(e,t,n){let r=this.move.keys,i=e=>e.bearing===void 0?(Number.isFinite(n)?n:0)+(e.turn??0):e.bearing,a=1;for(;a<r.length&&r[a].at<t;)a++;if(a>=r.length){let t=e[e.length-1];return{dist:t.dist,tilt:t.tilt,bearing:i(t),raise:t.raise}}let o=e[a-1],s=e[a],c=r[a-1].at,l=r[a].at,u=l>c?En(this.ease??r[a].ease??`smooth`,(t-c)/(l-c)):+(t>=l),d=i(o);return{dist:Math.exp(Math.log(o.dist)+(Math.log(s.dist)-Math.log(o.dist))*u),tilt:o.tilt+(s.tilt-o.tilt)*u,bearing:d+ei(d,i(s))*u,raise:o.raise+(s.raise-o.raise)*u}}step(e,t,n){let r=zr(this.move.aim.target,t,t.timeMs);if(!r)return null;this.keys&&(this.t+=e);let i=this.follower,a=t.exaggeration??1,o=(e,n)=>{let r=t.heightAt(e,n);return Number.isFinite(r)?r*a:NaN},s=i.step({place:[r.lon,r.lat,r.y],aim:[r.lon,r.lat,r.y],heading:Number.isFinite(r.heading)?r.heading:null},e,n.pose.dist),c=s.heading??NaN,l=s.place,u=s.aim,d={lon:l[0],lat:l[1],y:l[2]};this.keys??=this.resolve(n,d,c);let f=this.rigAt(this.keys,this.progress(),c),p=Gr(d,f),m=Math.max(0,this.need(p,t,o)-p.y);if(e>0){let t=1.678/(m>this.lift?.12:.7),n=Math.exp(-t*e),r=this.liftV+t*(this.lift-m);this.lift=m+(this.lift-m+r*e)*n,this.liftV=(this.liftV-t*r*e)*n,this.lifted=!0}else this.lifted||(this.lift=m);this.lift>1e-6&&(p={...p,y:p.y+this.lift});let h={lon:u[0],lat:u[1],y:u[2]};return Ur(h,Wr(p,h,(f.raise??0)+(this.move.aim.raise??0)),(e,t)=>{let n=o(e,t);return Number.isNaN(n)?r.g:n},n.fov,n.floor)}need(e,t,n){let r=n(e.lon,e.lat),i=t.exaggeration??1,a=Number.isFinite(r)?r+1:-1/0,o=this.move.over;if(o!==!1&&o!==void 0&&t.topIn){let n=111319.49,r=n*Math.cos(e.lat*G),s=t.topIn(e.lon-14/r,e.lat-14/n,e.lon+14/r,e.lat+14/n);s!==null&&Number.isFinite(s)&&(a=Math.max(a,s*i+o))}return a}poseAt(e,t,n,r=t.timeMs){let i=zr(this.move.aim.target,t,r);if(!i)return null;let a=this.keys??this.resolve(n,i,i.heading),o=this.rigAt(a,this.progress(e),i.heading),s=t.exaggeration??1,c=(e,n)=>{let r=t.heightAt(e,n);return Number.isFinite(r)?r*s:i.g};return Ur(i,{...o,raise:(o.raise??0)+(this.move.aim.raise??0)},c,n.fov,n.floor).pose}sameAim(e){let t=this.move.aim.target,n=e.aim.target;return t===n||typeof t==`object`&&typeof n==`object`&&!Array.isArray(t)&&!Array.isArray(n)&&`track`in t&&`track`in n&&t.track===n.track&&t.id===n.id}};let ni=null,ri=!1,ii=0,ai=0;const oi=/* @__PURE__ */ new Map;function si(){if(ni||ri)return ni;try{if(typeof Worker>`u`||typeof OffscreenCanvas>`u`||typeof Blob>`u`)return ri=!0,null;let e=URL.createObjectURL(new Blob([`self.onmessage = async (e) => {
  const { id, blob, raw, compose } = e.data;
  if (compose) {
    try {
      const { size, fill, parts } = compose, c = new OffscreenCanvas(size, size), g = c.getContext('2d', { willReadFrequently: true });
      g.fillStyle = fill; g.fillRect(0, 0, size, size);
      // every image here is the worker's own (sent, or a copy of the page's): closed once drawn, never left to the
      // garbage collector (a decoded image holds its pixels, on some phones in the graphics process)
      for (const p of parts) { g.globalAlpha = p.alpha; g.globalCompositeOperation = p.op; g.drawImage(p.im, p.sx, p.sy, p.sw, p.sh, 0, 0, size, size); p.im.close(); }
      const d = g.getImageData(0, 0, size, size).data;
      self.postMessage({ id, width: size, height: size, data: d }, [d.buffer]);
    } catch (err) { for (const p of compose.parts) { try { p.im.close(); } catch {} } self.postMessage({ id, error: String(err && err.message || err) }); }
    return;
  }
  try {
    const im = await createImageBitmap(blob, raw ? { colorSpaceConversion: 'none', premultiplyAlpha: 'none' } : { premultiplyAlpha: 'none' });
    const c = new OffscreenCanvas(im.width, im.height), g = c.getContext('2d', { willReadFrequently: true });
    g.drawImage(im, 0, 0); im.close();
    const d = g.getImageData(0, 0, c.width, c.height).data;
    self.postMessage({ id, width: c.width, height: c.height, data: d }, [d.buffer]);
  } catch (err) { self.postMessage({ id, error: String(err && err.message || err) }); }
};`],{type:`text/javascript`})),t=Math.max(1,Math.min(3,(typeof navigator<`u`&&navigator.hardwareConcurrency||4)-2));ni=[];try{for(let n=0;n<t;n++){let t=new Worker(e);t.onmessage=e=>{let n=oi.get(e.data.id);n&&n.worker===t&&(oi.delete(e.data.id),e.data.error?n.bad(new K(`E-DATA-002`,e.data.error,{fix:`check the image bytes and canvas support`})):n.ok({width:e.data.width,height:e.data.height,data:e.data.data}))},t.onerror=e=>ci(t,e.message),t.onmessageerror=()=>ci(t,`the worker response could not be read`),ni.push(t)}}finally{URL.revokeObjectURL(e)}}catch{for(let e of ni??[])e.terminate();ri=!0,ni=null}return ni}function ci(e,t){ni=ni?.filter(t=>t!==e)??null,ni?.length||(ni=null,ri=!0),e.terminate();let n=new K(`E-LOAD-006`,`image decode worker failed: ${t}`,{fix:`retry loading the image; check worker and canvas support`});for(let[t,r]of oi)r.worker===e&&(oi.delete(t),r.bad(n))}function li(e,t,n=[]){let r=++ai;return new Promise((i,a)=>{oi.set(r,{worker:e,ok:i,bad:a});try{e.postMessage({id:r,...t},n)}catch(t){ci(e,String(t))}})}function ui(){si()}async function di(e,t){let n=await createImageBitmap(e,t?{colorSpaceConversion:`none`,premultiplyAlpha:`none`}:{premultiplyAlpha:`none`}),r=new OffscreenCanvas(n.width,n.height),i=r.getContext(`2d`,{willReadFrequently:!0});return i.drawImage(n,0,0),n.close(),{width:r.width,height:r.height,data:i.getImageData(0,0,r.width,r.height).data}}function fi(e,t=!0){let n=si();return!n||ri?di(e,t):li(n[ii++%n.length],{blob:e,raw:t}).catch(n=>{if(n instanceof K&&n.code===`E-LOAD-006`)throw n;return di(e,t)})}function pi(e,t,n){let r=new OffscreenCanvas(e,e).getContext(`2d`,{willReadFrequently:!0});r.fillStyle=t,r.fillRect(0,0,e,e);for(let t of n)r.globalAlpha=t.alpha,r.globalCompositeOperation=t.op,r.drawImage(t.im,t.sx,t.sy,t.sw,t.sh,0,0,e,e),t.own&&t.im.close();return{width:e,height:e,data:r.getImageData(0,0,e,e).data}}function mi(e,t,n){let r=si();return!r||ri?Promise.resolve(pi(e,t,n)):li(r[ii++%r.length],{compose:{size:e,fill:t,parts:n}},n.filter(e=>e.own).map(e=>e.im))}function X(e){if(typeof e!=`string`||!/^[A-Za-z0-9_-][A-Za-z0-9._-]{0,127}$/.test(e))throw new K(`E-LOAD-004`,`unsafe scope name ${JSON.stringify(e)}`,{fix:`use letters, digits, '-', '_' and '.', not starting with a dot, at most 128 characters`});return e}const hi=()=>Date.now(),gi=e=>({name:e,created:hi(),at:hi(),bytes:0,items:0}),_i=`aslnut:`,vi=`https://aslnut.local/__scope__`,yi={source:`x-aslnut-source`,at:`x-aslnut-at`,bytes:`x-aslnut-bytes`,meta:`x-aslnut-meta`,status:`x-aslnut-status`};var bi=class{kind=`cache`;infos=/* @__PURE__ */ new Map;dirty=/* @__PURE__ */ new Set;flushT=null;open(e){return caches.open(_i+X(e))}async names(){return(await caches.keys()).filter(e=>e.startsWith(_i)).map(e=>e.slice(7))}async match(e,t){return await caches.has(_i+X(e))?await(await this.open(e)).match(t)??null:null}async get(e,t){let n=await this.match(e,t);if(!n)return null;let r=n.headers.get(yi.meta);return{status:Number(n.headers.get(yi.status)??n.status),type:n.headers.get(`content-type`)??``,data:await n.arrayBuffer(),source:n.headers.get(yi.source)??``,at:Number(n.headers.get(yi.at)??0),...r?{meta:JSON.parse(r)}:{}}}async has(e,t){return!!await this.match(e,t)}async head(e,t){let n=await this.match(e,t);return n?{bytes:Number(n.headers.get(yi.bytes)??0),source:n.headers.get(yi.source)??``,status:Number(n.headers.get(yi.status)??n.status)}:null}async put(e,t,n){let r=await this.open(e),i=!!await r.match(t),a={"content-type":n.type||`application/octet-stream`,[yi.source]:n.source,[yi.at]:String(n.at),[yi.bytes]:String(n.data.byteLength),[yi.status]:String(n.status)};n.meta&&Object.keys(n.meta).length&&(a[yi.meta]=JSON.stringify(n.meta)),await r.put(t,new Response(n.data,{status:200,headers:a}));let o=await this.info(e)??gi(e);i||(o.bytes+=n.data.byteLength,o.items++),o.at=hi(),this.infos.set(e,o),this.mark(e)}async keys(e){return await caches.has(_i+X(e))?(await(await this.open(e)).keys()).map(e=>e.url).filter(e=>e!==vi):[]}async info(e){let t=this.infos.get(e);if(t)return t;if(!await caches.has(_i+X(e)))return null;let n=await(await this.open(e)).match(vi),r=n?await n.json():await this.count(e);return this.infos.set(e,r),r}async setInfo(e,t){let n={...await this.info(e)??gi(e),...t,name:e};this.infos.set(e,n),this.mark(e),await this.flush()}async drop(e){X(e),this.infos.delete(e),this.dirty.delete(e),await caches.delete(_i+e)}async count(e){let t=await this.open(e),n=gi(e);for(let e of await t.keys()){if(e.url===vi)continue;let r=await t.match(e);n.items++,n.bytes+=Number(r?.headers.get(yi.bytes)??0)}return n}mark(e){this.dirty.add(e),this.flushT??=setTimeout(()=>{this.flush().catch(()=>{})},800)}async flush(){this.flushT&&=(clearTimeout(this.flushT),null);for(let e of[...this.dirty]){this.dirty.delete(e);let t=this.infos.get(e);t&&await(await this.open(e)).put(vi,new Response(JSON.stringify(t),{headers:{"content-type":`application/json`}}))}}close(){this.flushT&&=(clearTimeout(this.flushT),null)}},xi=class{kind=`memory`;m=/* @__PURE__ */ new Map;infos=/* @__PURE__ */ new Map;async names(){return[...this.m.keys()]}async get(e,t){return this.m.get(X(e))?.get(t)??null}async has(e,t){return!!this.m.get(X(e))?.has(t)}async head(e,t){let n=this.m.get(X(e))?.get(t);return n?{bytes:n.data.byteLength,source:n.source,status:n.status}:null}async put(e,t,n){let r=this.m.get(X(e));r||(r=/* @__PURE__ */ new Map,this.m.set(e,r));let i=this.infos.get(e)??gi(e);r.has(t)||(i.bytes+=n.data.byteLength,i.items++),r.set(t,n),i.at=hi(),this.infos.set(e,i)}async keys(e){return[...this.m.get(X(e))?.keys()??[]]}async info(e){return this.infos.get(e)??(this.m.has(e)?gi(e):null)}async setInfo(e,t){this.m.has(X(e))||this.m.set(e,/* @__PURE__ */ new Map),this.infos.set(e,{...this.infos.get(e)??gi(e),...t,name:e})}async drop(e){X(e),this.m.delete(e),this.infos.delete(e)}};const Si=`ASLNUTPK`,Ci=e=>new K(`E-LOAD-005`,`not a valid aslnut pack: ${e}`,{fix:`export the pack again with loader.export()`});function wi(e,t){let n=0,r={version:1,scope:X(e),created:Date.now(),entries:t.map(({key:e,e:t})=>{let r={key:e,status:t.status,type:t.type,source:t.source,at:t.at,...t.meta?{meta:t.meta}:{},offset:n,length:t.data.byteLength};return n+=t.data.byteLength,r})},i=new TextEncoder().encode(JSON.stringify(r)),a=/* @__PURE__ */ new Uint8Array(12);return a.set(new TextEncoder().encode(Si),0),new DataView(a.buffer).setUint32(8,i.byteLength,!0),new Blob([a,i,...t.map(({e})=>new Uint8Array(e.data))],{type:`application/x-aslnut-pack`})}var Ti=class e{name;scope;created;ix=/* @__PURE__ */ new Map;body;base;constructor(e,t,n,r){this.name=e,this.scope=t.scope,this.created=t.created,this.body=n,this.base=r;for(let e of t.entries)this.ix.set(e.key,e)}static async read(t,n){let r=t instanceof ArrayBuffer?t:await t.arrayBuffer();if(r.byteLength<12||new TextDecoder().decode(new Uint8Array(r,0,8))!==Si)throw Ci(`the header is missing`);let i=new DataView(r,8,4).getUint32(0,!0);if(12+i>r.byteLength)throw Ci(`the index runs past the end of the file`);let a;try{a=JSON.parse(new TextDecoder().decode(new Uint8Array(r,12,i)))}catch{throw Ci(`the index is not JSON`)}if(a.version!==1||!Array.isArray(a.entries))throw Ci(`unknown version ${String(a.version)}`);X(a.scope);let o=r.byteLength-12-i;for(let e of a.entries)if(typeof e.key!=`string`||!Number.isInteger(e.offset)||!Number.isInteger(e.length)||e.offset<0||e.length<0||e.offset+e.length>o)throw Ci(`entry ${JSON.stringify(e.key)} is out of bounds`);return new e(n??a.scope,a,r,12+i)}entries(){return[...this.ix.keys()].map(e=>({key:e,e:this.entry(this.ix.get(e))}))}entry(e){return{status:e.status,type:e.type,source:e.source,at:e.at,meta:e.meta,data:this.body.slice(this.base+e.offset,this.base+e.offset+e.length)}}async get(e){let t=this.ix.get(e);return t?this.entry(t):null}async has(e){return this.ix.has(e)}keys(){return[...this.ix.keys()]}get bytes(){return this.body.byteLength-this.base}};const Ei=Object.freeze({show:!0,prefetch:!1,keep:!1,export:!1}),Di=Object.freeze({show:!0,prefetch:!0,keep:!0,export:!0}),Oi=(e,t=Di)=>({...t,...e}),ki=()=>Object.assign(/* @__PURE__ */ Error(`the load was abandoned`),{name:`AbortError`,retry:!1});var Ai=class extends K{url;mode;constructor(e,t){super(`E-LOAD-001`,`${e} is not ready (${t} mode: no network)`,{fix:`prepare it first: loader.ensure(urls) in live mode, or import its pack`}),this.name=`LoaderMiss`,this.url=e,this.mode=t}};const ji={esri:{match:/arcgisonline\.com|arcgis\.com/,terms:Ei,concurrency:6},osm:{match:/tile\.openstreetmap\.org/,terms:Ei,concurrency:2},mapbox:{match:/api\.mapbox\.com/,terms:Ei},mapzen:{match:/elevation-tiles-prod/,terms:Di,store:`scope`,concurrency:8},sentinel:{match:/tiles\.maps\.eox\.at/,terms:Di,store:`scope`,concurrency:12},default:{terms:Di,store:`scope`}},Mi=1048576,Ni=typeof window>`u`&&!!globalThis.process?.versions?.node,Pi=(e,t)=>typeof e==`number`?e:e?Number(/[\d.]+/.exec(e)?.[0]??0)*(/g/i.test(e)?1024*Mi:/k/i.test(e)?1024:Mi):t,Fi=e=>new Promise(t=>setTimeout(t,e)),Ii=()=>Date.now();var Li=class e{scope;_mode;store;rules;packs=[];mem=/* @__PURE__ */ new Map;memBytes=0;budget;flying=/* @__PURE__ */ new Map;holds=/* @__PURE__ */ new Map;hosts=/* @__PURE__ */ new Map;ev={};stat=e.blankStats();o;ready;disposed=!1;aborts=/* @__PURE__ */ new Set;constructor(e={}){this.o=e,this.scope=X(e.scope??`default`),this._mode=e.mode??`live`;let t=typeof navigator<`u`?navigator.deviceMemory:void 0;this.budget=Pi(e.memory,Ni?512*Mi:Math.min(192,(t??8)*24)*Mi),this.store=e.store===void 0?typeof caches<`u`?new bi:new xi:e.store;let n=Object.entries(e.sources??{}).filter(([e])=>e!=="default"),r=Object.entries(ji).filter(([t])=>t!=="default"&&!(e.sources&&t in e.sources));this.rules=[...n,...r,[`default`,{...ji.default,...e.sources?.default??{}}]];for(let t of e.packs??[])this.addPack(t);this.ready=this.housekeeping().catch(()=>{})}static blankStats(){return{tiers:{memory:0,shared:0,pack:0,store:0,network:0},bytesNetwork:0,misses:0,errors:0,kept:0,refused:0,inflight:0,byKind:{},bySource:{}}}get mode(){return this._mode}set mode(e){this._mode=e}source(e,t){let n=this.rules.findIndex(([t])=>t===e);return n>=0?this.rules[n]=[e,{...this.rules[n][1],...t}]:this.rules.splice(Math.max(0,this.rules.length-1),0,[e,t]),this}ruleOf(e,t){if(t){let e=this.rules.find(([e])=>e===t);if(e)return e}for(let[t,n]of this.rules){let r=n.match;if(r&&(typeof r==`string`?e.includes(r):r instanceof RegExp?r.test(e):r(e)))return[t,n]}return[`default`,this.rules.find(([e])=>e==="default")[1]]}termsOf(e,t){return Oi(this.ruleOf(e,t)[1].terms)}can(e,t,n){return this.termsOf(t,n)[e]}keyOf(e){let t=typeof e==`string`?e:e.url,n=typeof e==`string`?void 0:e.range,r=!Ni&&typeof location<`u`?new URL(t,location.href).href:t;return n?`${r}${r.includes(`?`)?`&`:`?`}__range=${n[0]}-${n[1]}`:r}idOf(e,t){if(!t?.headers&&!t?.credentials)return e;let n=[];return new Headers(t.headers).forEach((e,t)=>n.push(`${t}:${e}`)),`${e}#${t.credentials??``}#${n.sort().join(`|`)}`}async get(e,t={}){if(this.disposed)throw new K(`E-WORLD-003`,`the loader was disposed`,{fix:`make a new loader`});let n=t.kind??`bytes`,r=this.keyOf(e),i=`${n}|${this.idOf(r,t.init)}`,[a,o]=this.ruleOf(r,t.source),s=Oi(o.terms),c=o.policy??`cacheFirst`,l=X(t.scope??this.scope);if(!s.show)throw this.refuse(r,a,`show`);if(t.prefetch&&!s.prefetch)throw this.refuse(r,a,`prefetch`);if(t.keep&&!s.keep)throw this.refuse(r,a,`keep`);let u=this.mem.get(i);if(u&&this.memoryAnswers(u,c,l,!!t.keep,s))return this.mem.delete(i),this.mem.set(i,u),u.seen=Ii(),this.stat.tiers.memory++,u.v;let d=`${i}|${c}|${this._mode}|${l}|${+!!t.keep}`,f=this.flying.get(d);if(f)return this.stat.tiers.shared++,this.waitFor(f,d,t.signal);if(t.signal?.aborted)throw ki();(n===`pixels`||n===`imagePixels`)&&ui();let p={ctrl:new AbortController,n:0,firm:!1},m=(async()=>{let s=Ii(),u=await this.resolve(e,r,a,o,c,l,{...t,signal:p.ctrl.signal}),d=new Set(u?.from?[u.from]:[]);if(!u||u.status===404||u.status===204)return this.remember(i,null,64,d),null;let f=await Bi(n,u,r);this.remember(i,f,Vi(n,f,u),d);let m=this.stat.byKind[n]??={count:0,bytes:0};return m.count++,m.bytes+=u.data.byteLength,this.emit(`load`,{url:r,kind:n,tier:u.tier??`network`,source:a,bytes:u.data.byteLength,ms:Ii()-s}),f})().finally(()=>{this.flying.delete(d),this.holds.delete(d)});return this.flying.set(d,m),this.holds.set(d,p),this.waitFor(m,d,t.signal)}waitFor(e,t,n){let r=this.holds.get(t);return r?n?n.aborted?Promise.reject(ki()):(r.n++,new Promise((t,i)=>{let a=()=>{i(ki()),--r.n<=0&&!r.firm&&this._mode!==`record`&&r.ctrl.abort()};n.addEventListener(`abort`,a,{once:!0}),e.then(e=>{n.removeEventListener(`abort`,a),t(e)},e=>{n.removeEventListener(`abort`,a),i(e)})})):(r.firm=!0,e):e}memoryAnswers(e,t,n,r,i){let a=this._mode===`replay`||this._mode===`offline`;if(t===`networkOnly`||t===`networkFirst`&&!a)return!1;if(a){for(let t of this.readScopes(n))if(e.kept.has(t))return!0;for(let t of this.packs)if(e.kept.has(`pack:${t.name}`))return!0;return!1}return(this._mode===`record`||r)&&i.keep?e.kept.has(n)||[...e.kept].some(e=>e.startsWith(`pack:`)):!0}refuse(e,t,n){return this.stat.refused++,this.emit(`refuse`,{url:e,source:t,capability:n}),new K(`E-LOAD-002`,`the terms of '${t}' do not allow ${n===`show`?`showing`:n===`prefetch`?`prefetching`:n===`keep`?`keeping`:`exporting`} ${e}`,{where:t,fix:n===`prefetch`||n===`keep`?`use a provider that allows it (terrain.mapzen(), imagery.sentinel(), your own tiles), or hold a licence and set its terms`:`see DATA-LICENCES.md`})}async objectURL(e,t){let n=await this.get(e,{kind:`entry`});if(!n)throw new K(`E-LOAD-001`,`nothing at ${e}`,{fix:`check the URL`});return URL.createObjectURL(new Blob([n.data],{type:t??n.type}))}async resolve(e,t,n,r,i,a,o){await this.ready;let s=this.stat.bySource[n]??={count:0,network:0,bytes:0};s.count++;let c=this._mode===`replay`||this._mode===`offline`||i===`cacheOnly`;for(let e of this.packs){let n=await e.get(t).catch(()=>null);if(n)return this.stat.tiers.pack++,{...n,tier:`pack`,from:`pack:${e.name}`}}let l=null,u=async()=>{if(!this.store||i===`networkOnly`)return null;for(let e of this.readScopes(a)){let n=await this.store.get(e,t).catch(()=>null);if(n){if(r.maxAge&&Ii()-n.at>r.maxAge&&!c)l??={...n,tier:`store`,from:e};else return this.stat.tiers.store++,{...n,tier:`store`,from:e}}}return null};if(i!==`networkFirst`||c){let e=await u();if(e)return(this._mode===`record`||o.keep)&&e.from!==a&&Oi(r.terms).keep&&await this.keep(t,e,r,a),e}if(c){if(l)return l;throw this.stat.misses++,this.emit(`miss`,{url:t,mode:this._mode,source:n,scope:a}),new Ai(t,this._mode)}let d;try{d=await this.network(e,n,r,o.priority??`now`,o.init,o.signal)}catch(e){if(o.signal?.aborted)throw ki();let n=i===`networkFirst`?await u():null;if(n)return n;if(l)return l;throw this.stat.errors++,this.emit(`error`,{url:t,error:String(e.message??e)}),e}this.stat.tiers.network++,s.network++,s.bytes+=d?.data.byteLength??0,this.stat.bytesNetwork+=d?.data.byteLength??0;let f;return d&&(this._mode===`record`||o.keep||r.store===`persist`)&&(f=await this.keep(t,d,r,a)),d?{...d,tier:`network`,from:f}:null}extra=/* @__PURE__ */ new Set;readScopes(e=this.scope){return[.../* @__PURE__ */ new Set([e,`session`,`persist`,...this.extra])]}read(e){return this.extra.add(X(e)),this}async keep(e,t,n,r){if(!this.store)return;if(!Oi(n.terms).keep){this.stat.refused++,this.emit(`refuse`,{url:e,source:t.source,capability:`keep`});return}let i=n.store===`session`?`session`:n.store===`persist`?`persist`:r;try{return await this.store.put(i,e,t),this.stat.kept++,i}catch(t){this.emit(`error`,{url:e,error:`keep: ${t.message}`});return}}network(e,t,n,r,i,a){let o=typeof e==`string`?e:e.url,s=typeof e==`string`?void 0:e.range;s&&Ri(s,o);let c=/^[a-z]+:\/\/([^/]+)/i.exec(o)?.[1]??`local`,l=n.concurrency??this.o.concurrency??6,u=this.hosts.get(c)??{active:0,q:{now:[],soon:[],later:[]}};return this.hosts.set(c,u),new Promise((e,n)=>{let d=async()=>{if(a?.aborted)n(ki()),this.pump(c,l);else{u.active++,this.stat.inflight++;try{for(let n=0;;n++)try{e(await this.fetchOnce(o,s,t,i,a));return}catch(e){if(!(!this.disposed&&!a?.aborted&&n<(this.o.retries??3)&&e.retry!==!1))throw e;await Fi(300*2**n)}}catch(e){n(e)}finally{u.active--,this.stat.inflight--,this.pump(c,l)}}};u.q[r].push(()=>void d()),this.pump(c,l)})}pump(e,t){let n=this.hosts.get(e);for(;n.active<t;){let e=n.q.now.shift()??n.q.soon.shift()??(n.active<Math.max(1,t-1)?n.q.later.shift():void 0);if(!e)return;e()}}async fetchOnce(e,t,n,r,i){if(Ni&&!/^[a-z]+:\/\//i.test(e)||e.startsWith(`file://`))return Hi(e,t,n);let a=this.o.fetch??fetch,o=new Headers(r?.headers);t&&o.set(`Range`,`bytes=${t[0]}-${t[0]+t[1]-1}`);let s=new AbortController,c=setTimeout(()=>s.abort(),this.o.timeout??3e4);this.aborts.add(s);let l=()=>s.abort();i?.addEventListener(`abort`,l,{once:!0});let u,d;try{try{u=await a(e,{...r,headers:o,mode:r?.mode??`cors`,signal:s.signal})}catch(t){throw Object.assign(/* @__PURE__ */ Error(`network: ${t.message} ${e}`),{retry:!this.disposed})}if(u.status===404||u.status===204||u.status===410)return{status:404,type:``,data:/* @__PURE__ */ new ArrayBuffer(0),source:n,at:Ii()};if(u.status===416)throw Object.assign(new K(`E-LOAD-003`,`the range ${t?.join(`+`)} is outside ${e}`,{fix:`check the offset and length`}),{retry:!1});if(!u.ok)throw Object.assign(/* @__PURE__ */ Error(`${u.status} ${e.replace(/access_token=[^&]+/,`access_token=…`)}`),{retry:u.status>=500||u.status===429});d=await u.arrayBuffer()}finally{clearTimeout(c),this.aborts.delete(s),i?.removeEventListener(`abort`,l)}t&&(d=zi(u,d,t,e));let f={};return u.headers.forEach((e,t)=>{/^x-/i.test(t)&&!/^x-aslnut/i.test(t)&&(f[t.toLowerCase()]=e)}),{status:200,type:u.headers.get(`content-type`)??``,data:d,source:n,at:Ii(),...Object.keys(f).length?{meta:f}:{}}}remember(e,t,n,r){if(n>this.budget/8)return;let i=this.mem.get(e);if(i){this.memBytes-=i.bytes,this.mem.delete(e);for(let e of i.kept)r.add(e)}this.mem.set(e,{v:t,bytes:n,kept:r,seen:Ii()}),this.memBytes+=n,this.trim()}trim(){if(this.memBytes<=this.budget)return;let e=this.o.evict,t=Ii(),n=typeof e==`function`?[...this.mem].map(([n,r])=>({k:n,x:r,s:e({key:n,bytes:r.bytes,seenMsAgo:t-r.seen})})).sort((e,t)=>e.s-t.s).map(({k:e,x:t})=>[e,t]):this.mem;for(let[e,t]of n){if(this.memBytes<=this.budget)break;this.mem.delete(e),this.memBytes-=t.bytes,this.emit(`evict`,{key:e,bytes:t.bytes})}}setMemory(e,t){return this.budget=Pi(e,this.budget),t!==void 0&&(this.o={...this.o,evict:t}),this.trim(),this}forget(){this.mem.clear(),this.memBytes=0}addPack(e){return this.packs=[e,...this.packs.filter(t=>t.name!==e.name)],this}removePack(e){return this.packs=this.packs.filter(t=>t.name!==e),this}get packList(){return[...this.packs]}async export(e=this.scope){if(!this.store)throw new K(`E-LOAD-002`,`no store to export from`,{fix:`give the loader a store`});let t=[],n=[];for(let r of await this.store.keys(X(e))){let i=await this.store.get(e,r);i&&(this.can(`export`,r,this.rules.some(([e])=>e===i.source)?i.source:void 0)?t.push({key:r,e:i}):(n.push(r),this.emit(`refuse`,{url:r,source:i.source,capability:`export`})))}return{blob:wi(e,t),items:t.length,refused:n}}async import(e,t={}){let n=await Ti.read(e,t.name);if((t.as??`pack`)===`pack`)return this.addPack(n),{scope:n.scope,items:n.keys().length};if(!this.store)throw new K(`E-LOAD-002`,`no store to import into`,{fix:`give the loader a store, or import as: 'pack'`});let r=X(t.scope??n.scope);for(let{key:e,e:t}of n.entries())await this.store.put(r,e,t);return{scope:r,items:n.keys().length}}async scopes(){if(!this.store)return[];let e=[];for(let t of await this.store.names()){let n=await this.store.info(t);n&&e.push(n)}return e.sort((e,t)=>t.at-e.at)}async scopeRules(e,t){await this.store?.setInfo(X(e),t)}async pin(e,t=!0){await this.store?.setInfo(X(e),{pinned:t})}async clear(e){if(!this.store)return 0;let t=e?[X(e)]:await this.store.names();for(let e of t)await this.store.drop(e);return this.forget(),t.length}async housekeeping(){if(!this.store)return;let e=Ni;try{typeof sessionStorage<`u`&&(e=!sessionStorage.getItem(`aslnut.session`),sessionStorage.setItem(`aslnut.session`,`1`))}catch{}e&&await this.store.drop(`session`);let t=Ii()-(this.o.maxAge??6048e5);for(let e of await this.store.names()){let n=await this.store.info(e);n&&!n.pinned&&!n.persist&&e!==`persist`&&(n.maxAge?n.at<Ii()-n.maxAge:n.at<t)&&await this.store.drop(e)}}async coverage(e,t=this.scope){let n=Array.isArray(e)?e:e.items.map(e=>e.url),r=[],i=[];for(let e of n){let n=this.keyOf(e),a=0;for(let e of this.packs){let t=await e.get(n).catch(()=>null);if(t){a=t.status;break}}if(!a&&this.store)for(let e of this.readScopes(t)){let t=await this.store.head(e,n);if(t){a=t.status;break}}a!==200&&a!==206&&(a?i:r).push(e)}let a=n.length-r.length-i.length;return{have:a,total:n.length,fraction:n.length?a/n.length:1,missing:r,absent:i}}async ensure(e,t={}){let n=X(t.scope??this.scope),r=await this.coverage(e,n),i={fetched:0,already:r.have,absent:r.absent.length,failed:0,refused:0,bytes:0,problems:[]},a=r.have+r.absent.length;t.onProgress?.(a,r.total,0);let o=[...r.missing];return await Promise.all(Array.from({length:t.concurrency??12},async()=>{for(;o.length;){let e=o.shift(),s=this.termsOf(this.keyOf(e));if(!s.prefetch||!s.keep)i.refused++,i.problems.push({url:e,why:`terms: ${s.prefetch?`no keep`:`no prefetch`}`});else try{let t=await this.get(e,{kind:`entry`,scope:n,keep:!0,prefetch:!0,priority:`later`});t&&(t.status===200||t.status===206)?(i.fetched++,i.bytes+=t.data.byteLength):(i.absent++,i.problems.push({url:e,why:`not found (404)`}))}catch(t){i.failed++,i.problems.push({url:e,why:String(t.message??t)})}t.onProgress?.(++a,r.total,i.bytes),this.emit(`progress`,{phase:`ensure`,done:a,total:r.total,bytes:i.bytes})}})),i}async manifest(e=this.scope){let t=[];if(this.store)for(let n of await this.store.keys(X(e))){let r=await this.store.head(e,n);r&&t.push({url:n,...r})}return{scope:e,created:Date.now(),bytes:t.reduce((e,t)=>e+t.bytes,0),items:t}}stats(){return{mode:this._mode,scope:this.scope,tiers:{...this.stat.tiers},bytesNetwork:this.stat.bytesNetwork,misses:this.stat.misses,errors:this.stat.errors,kept:this.stat.kept,refused:this.stat.refused,inflight:this.stat.inflight,memory:{bytes:this.memBytes,items:this.mem.size,budget:this.budget},byKind:structuredClone(this.stat.byKind),bySource:structuredClone(this.stat.bySource),packs:this.packs.map(e=>e.name)}}resetStats(){let t=this.stat.inflight;this.stat=e.blankStats(),this.stat.inflight=t}on(e,t){let n=this.ev,r=n[e]??=/* @__PURE__ */ new Set;return r.add(t),()=>r.delete(t)}emit(e,t){let n=this.ev[e];if(n)for(let e of n)try{e(t)}catch{}}dispose(){this.disposed=!0;for(let e of this.aborts)e.abort();this.aborts.clear();for(let e of this.hosts.values())e.q.now.length=0,e.q.soon.length=0,e.q.later.length=0;for(let e of this.mem.values())typeof ImageBitmap<`u`&&e.v instanceof ImageBitmap&&e.v.close();this.forget(),this.ev={},this.store instanceof bi&&this.store.close()}};function Ri(e,t){if(!Number.isInteger(e[0])||!Number.isInteger(e[1])||e[0]<0||e[1]<=0)throw new K(`E-LOAD-003`,`a byte range must be [offset ≥ 0, length > 0] integers (got [${e.join(`, `)}])`,{where:t,fix:`pass [offset, length]`})}function zi(e,t,n,r){let[i,a]=n;if(e.status===206){let n=e.headers.get(`content-range`),o=e.type===`cors`||typeof location<`u`&&new URL(r,location.href).origin!==location.origin;if(n===null&&o&&t.byteLength===a)return t;let s=/^bytes (\d+)-(\d+)\/(\d+|\*)$/.exec(e.headers.get(`content-range`)??``),c=s&&s[3]!==`*`?Number(s[3]):1/0,l=Math.min(i+a,c)-1;if(!s||Number(s[1])!==i||Number(s[2])!==l||t.byteLength!==l-i+1)throw Object.assign(new K(`E-LOAD-003`,`asked bytes ${i}-${i+a-1}, got '${e.headers.get(`content-range`)??`no Content-Range`}' and ${t.byteLength} bytes`,{where:r,fix:`answer Range requests exactly and expose the header with Access-Control-Expose-Headers: Content-Range`}),{retry:!1});return t}if(t.byteLength<=i)throw Object.assign(new K(`E-LOAD-003`,`the range starts at ${i}, past the end (${t.byteLength} bytes)`,{where:r}),{retry:!1});return t.slice(i,i+a)}async function Bi(e,t,n){switch(e){case`entry`:return t;case`bytes`:return t.data;case`text`:return new TextDecoder().decode(t.data);case`json`:{let e=t.data;return/\.gz$/i.test(n.split(`?`)[0])&&new Uint8Array(e,0,2)[0]===31&&(e=await new Response(new Blob([e]).stream().pipeThrough(new DecompressionStream(`gzip`))).arrayBuffer()),JSON.parse(new TextDecoder().decode(e))}case`image`:case`imageRaw`:if(typeof createImageBitmap>`u`)throw new K(`E-LOAD-001`,`images need a browser (createImageBitmap)`,{where:n,fix:`load kind: 'bytes' in Node`});return createImageBitmap(new Blob([t.data],{type:t.type}),e===`imageRaw`?{colorSpaceConversion:`none`,premultiplyAlpha:`none`}:{});case`pixels`:return fi(new Blob([t.data],{type:t.type}));case`imagePixels`:return fi(new Blob([t.data],{type:t.type}),!1)}}function Vi(e,t,n){if(e===`image`||e===`imageRaw`){let e=t;return e.width*e.height*4}return e===`pixels`||e===`imagePixels`?t.data.byteLength:e===`json`?n.data.byteLength*(/gzip/.test(n.type)?12:3):e===`text`?n.data.byteLength*2:n.data.byteLength}async function Hi(e,t,n){let r=await import(
/* @vite-ignore */
`node:fs/promises`),i=decodeURIComponent(e.replace(/^file:\/\//,``));try{let a;if(t){let n=await r.open(i,`r`);try{a=new Uint8Array(t[1]);let{bytesRead:r}=await n.read(a,0,t[1],t[0]);if(!r)throw Object.assign(new K(`E-LOAD-003`,`the range starts at ${t[0]}, past the end`,{where:e}),{retry:!1});a=a.subarray(0,r)}finally{await n.close()}}else a=await r.readFile(i);return{status:200,type:``,data:a.buffer.slice(a.byteOffset,a.byteOffset+a.byteLength),source:n,at:Ii()}}catch(e){if(e.code===`ENOENT`)return{status:404,type:``,data:/* @__PURE__ */ new ArrayBuffer(0),source:n,at:Ii()};throw Object.assign(e,{retry:!1})}}const Ui=(e={})=>new Li(e),Wi={graphics:0,buffers:0,textures:0,targets:0,programs:0,bytes:0,made:0};function Gi(){return{...Wi}}const Ki={open(){Wi.graphics++,Wi.made++},close(){Wi.graphics--},add(e,t=0){Wi[e]++,Wi.bytes+=t},remove(e,t=0){Wi[e]--,Wi.bytes-=t}};var qi=class{m=/* @__PURE__ */ new Map;add(e,t){let n=this.m.get(e);n||this.m.set(e,n={bytes:0,count:0}),n.bytes+=t,n.count++}remove(e,t){let n=this.m.get(e);n&&(n.bytes-=t,--n.count<=0&&this.m.delete(e))}list(){return[...this.m].map(([e,t])=>({label:e,bytes:t.bytes,count:t.count})).sort((e,t)=>t.bytes-e.bytes)}};const Ji=e=>new K(`E-GPU-003`,`texture '${e.label??`texture`}' was not made to be read back`,{where:e.label,fix:`make it with texture({ …, renderable: true }) (or growable) to read it back`}),Yi={vertex:`#version 300 es
void main() {
  float x = float((gl_VertexID << 1) & 2) * 2.0 - 1.0;
  float y = float(gl_VertexID & 2) * 2.0 - 1.0;
  gl_Position = vec4(x, y, 0.0, 1.0);
}
`,fragment:`#version 300 es
precision highp float;
uniform sampler2D pic;
out vec4 outColor;
float luma(vec3 c) {
  return dot(c, vec3(0.299, 0.587, 0.114));
}
vec3 at(vec2 uv) {
  return textureLod(pic, uv, 0.0).rgb;
}
vec3 fxaa(vec2 uv, vec2 px) {
  float nw = luma(at(uv + vec2(-1.0, -1.0) * px));
  float ne = luma(at(uv + vec2(1.0, -1.0) * px));
  float sw = luma(at(uv + vec2(-1.0, 1.0) * px));
  float se = luma(at(uv + vec2(1.0, 1.0) * px));
  float m = luma(at(uv));
  float lo = min(m, min(min(nw, ne), min(sw, se)));
  float hi = max(m, max(max(nw, ne), max(sw, se)));
  vec2 dir = vec2(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
  float reduce = max((nw + ne + sw + se) * 0.03125, 0.0078125);
  float rcp = 1.0 / (min(abs(dir.x), abs(dir.y)) + reduce);
  dir = clamp(dir * rcp, vec2(-8.0), vec2(8.0)) * px;
  vec3 a = 0.5 * (at(uv + dir * (1.0 / 3.0 - 0.5)) + at(uv + dir * (2.0 / 3.0 - 0.5)));
  vec3 b = a * 0.5 + 0.25 * (at(uv - dir * 0.5) + at(uv + dir * 0.5));
  float lb = luma(b);
  if (lb < lo || lb > hi) {
    return a;
  }
  return b;
}
void main() {
  vec2 size = vec2(textureSize(pic, 0));
  outColor = vec4(fxaa(gl_FragCoord.xy / size, 1.0 / size), 1.0);
}
`},Xi={vertex:`#version 300 es
void main() {
  float x = float((gl_VertexID << 1) & 2) * 2.0 - 1.0;
  float y = float(gl_VertexID & 2) * 2.0 - 1.0;
  gl_Position = vec4(x, y, 0.0, 1.0);
}
`,layer:`#version 300 es
precision highp float;
precision highp sampler2DArray;
uniform sampler2DArray src;
uniform int layer;
uniform ivec2 at;
out vec4 outColor;
void main() {
  outColor = texelFetch(src, ivec3(ivec2(gl_FragCoord.xy) - at, layer), 0);
}
`,plane:`#version 300 es
precision highp float;
uniform sampler2D src;
uniform ivec2 at;
out vec4 outColor;
void main() {
  outColor = texelFetch(src, ivec2(gl_FragCoord.xy) - at, 0);
}
`};function Zi(e){let t=e.getContext(`webgl2`,{antialias:!1,depth:!0,stencil:!1,alpha:!1,premultipliedAlpha:!1,preserveDrawingBuffer:!1,powerPreference:`high-performance`});if(!t)throw Error(`the canvas gave no WebGL2 context`);let n=t.getExtension(`WEBGL_debug_renderer_info`),r={api:`webgl2`,compute:!1,timer:!!t.getExtension(`EXT_disjoint_timer_query_webgl2`),floatTargets:!!t.getExtension(`EXT_color_buffer_float`),maxTex:t.getParameter(t.MAX_TEXTURE_SIZE),maxLayers:t.getParameter(t.MAX_ARRAY_TEXTURE_LAYERS),vertexTextures:t.getParameter(t.MAX_VERTEX_TEXTURE_IMAGE_UNITS),f16:!1,device:n?String(t.getParameter(n.UNMASKED_RENDERER_WEBGL)):``},i=e.width,a=e.height,o=0,s=!1,c=0,l=new qi,u={draws:0,passes:0,uploadBytes:0,uploads:0,gpuMs:null,uploadMaxMs:0},d=0,f=0,p=t.getExtension(`EXT_disjoint_timer_query_webgl2`),m=null,h=!1,g=()=>{p&&!m&&(m=t.createQuery(),m&&(t.beginQuery(p.TIME_ELAPSED_EXT,m),h=!0))},_=()=>{h&&p&&(t.endQuery(p.TIME_ELAPSED_EXT),h=!1)},v=()=>{p&&m&&!h&&t.getQueryParameter(m,t.QUERY_RESULT_AVAILABLE)&&(t.getParameter(p.GPU_DISJOINT_EXT)||(u.gpuMs=+(t.getQueryParameter(m,t.QUERY_RESULT)/1e6).toFixed(2)),t.deleteQuery(m),m=null)},y=e=>{let t=performance.now();try{return e()}finally{let e=performance.now()-t;e>f&&(f=e)}},b=(e,n,r=!1)=>{let i=t.createFramebuffer(),a=t.createRenderbuffer(),o=r?null:t.createRenderbuffer(),s=r?t.createTexture():void 0;return o&&(t.bindRenderbuffer(t.RENDERBUFFER,o),t.renderbufferStorage(t.RENDERBUFFER,t.RGBA8,e,n)),s&&(t.bindTexture(t.TEXTURE_2D,s),t.texStorage2D(t.TEXTURE_2D,1,t.RGBA8,e,n),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MIN_FILTER,t.NEAREST),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MAG_FILTER,t.NEAREST),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_WRAP_S,t.CLAMP_TO_EDGE),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_WRAP_T,t.CLAMP_TO_EDGE),t.bindTexture(t.TEXTURE_2D,null)),t.bindRenderbuffer(t.RENDERBUFFER,a),t.renderbufferStorage(t.RENDERBUFFER,t.DEPTH_COMPONENT24,e,n),t.bindRenderbuffer(t.RENDERBUFFER,null),t.bindFramebuffer(t.FRAMEBUFFER,i),o?t.framebufferRenderbuffer(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,t.RENDERBUFFER,o):t.framebufferTexture2D(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,t.TEXTURE_2D,s,0),t.framebufferRenderbuffer(t.FRAMEBUFFER,t.DEPTH_ATTACHMENT,t.RENDERBUFFER,a),t.bindFramebuffer(t.FRAMEBUFFER,null),{fb:i,color:o,colorTex:s,depth:a,w:e,h:n,copyFb:null,copy:null}},x=e=>{t.deleteFramebuffer(e.fb),e.color&&t.deleteRenderbuffer(e.color),e.colorTex&&t.deleteTexture(e.colorTex),t.deleteRenderbuffer(e.depth),e.copyFb&&t.deleteFramebuffer(e.copyFb),e.copy&&t.deleteTexture(e.copy),e.keptFb&&t.deleteFramebuffer(e.keptFb),e.kept&&t.deleteTexture(e.kept)},S=e=>{e.copy||(e.copy=t.createTexture(),t.bindTexture(t.TEXTURE_2D,e.copy),t.texStorage2D(t.TEXTURE_2D,1,t.DEPTH_COMPONENT24,e.w,e.h),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MIN_FILTER,t.NEAREST),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MAG_FILTER,t.NEAREST),t.bindTexture(t.TEXTURE_2D,null),e.copyFb=t.createFramebuffer(),t.bindFramebuffer(t.FRAMEBUFFER,e.copyFb),t.framebufferTexture2D(t.FRAMEBUFFER,t.DEPTH_ATTACHMENT,t.TEXTURE_2D,e.copy,0)),t.bindFramebuffer(t.READ_FRAMEBUFFER,e.fb),t.bindFramebuffer(t.DRAW_FRAMEBUFFER,e.copyFb),t.blitFramebuffer(0,0,e.w,e.h,0,0,e.w,e.h,t.DEPTH_BUFFER_BIT,t.NEAREST),t.bindFramebuffer(t.FRAMEBUFFER,e.fb)},C=e=>{e.kept||(e.kept=t.createTexture(),t.bindTexture(t.TEXTURE_2D,e.kept),t.texStorage2D(t.TEXTURE_2D,1,t.DEPTH_COMPONENT24,e.w,e.h),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MIN_FILTER,t.NEAREST),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MAG_FILTER,t.NEAREST),t.bindTexture(t.TEXTURE_2D,null),e.keptFb=t.createFramebuffer(),t.bindFramebuffer(t.FRAMEBUFFER,e.keptFb),t.framebufferTexture2D(t.FRAMEBUFFER,t.DEPTH_ATTACHMENT,t.TEXTURE_2D,e.kept,0)),t.bindFramebuffer(t.READ_FRAMEBUFFER,e.fb),t.bindFramebuffer(t.DRAW_FRAMEBUFFER,e.keptFb),t.blitFramebuffer(0,0,e.w,e.h,0,0,e.w,e.h,t.DEPTH_BUFFER_BIT,t.NEAREST),t.bindFramebuffer(t.FRAMEBUFFER,e.fb)},w=null,T=null,E=null,D=null,O=()=>{if(E)return E;let e=t.createProgram(),n=t.createShader(t.VERTEX_SHADER),r=t.createShader(t.FRAGMENT_SHADER);return t.shaderSource(n,Yi.vertex),t.compileShader(n),t.shaderSource(r,Yi.fragment),t.compileShader(r),t.attachShader(e,n),t.attachShader(e,r),t.linkProgram(e),t.getProgramParameter(e,t.LINK_STATUS)?(t.useProgram(e),t.uniform1i(t.getUniformLocation(e,`pic`),0),D=t.createVertexArray(),E=e):(t.deleteProgram(e),null)},k=null,A=(e,n)=>{if(k&&k.w===e&&k.h===n)return k;k&&t.deleteTexture(k.tex);let r=t.createTexture();return t.bindTexture(t.TEXTURE_2D,r),t.texStorage2D(t.TEXTURE_2D,1,t.RGBA8,e,n),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MIN_FILTER,t.NEAREST),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MAG_FILTER,t.NEAREST),t.bindTexture(t.TEXTURE_2D,null),k={tex:r,w:e,h:n}},j={layer:null,plane:null,linking:{},vao:null,uLayer:null,uAt:[null,null]},M=e=>{if(j[e]||j.linking[e])return;let n=t.createProgram(),r=oe(t.VERTEX_SHADER,Xi.vertex),i=oe(t.FRAGMENT_SHADER,Xi[e]);t.attachShader(n,r),t.attachShader(n,i),t.linkProgram(n),t.deleteShader(r),t.deleteShader(i),j.linking[e]=n},N=e=>{let n=e===t.TEXTURE_2D_ARRAY?`layer`:`plane`,r=j[n];if(r)return r;M(n);let i=j.linking[n];return delete j.linking[n],t.getProgramParameter(i,t.LINK_STATUS)?(t.useProgram(i),t.uniform1i(t.getUniformLocation(i,`src`),0),n===`layer`&&(j.uLayer=t.getUniformLocation(i,`layer`)),j.uAt[n===`layer`?0:1]=t.getUniformLocation(i,`at`),t.useProgram(null),j.vao??=t.createVertexArray(),j[n]=i):(t.deleteProgram(i),null)},P=()=>{for(let e=0;e<8&&t.getError()!==t.NO_ERROR;e++);},F=(e,n,r,i,a,o,s,c)=>{let l=N(e.target);if(!l)return!1;let u=t.createFramebuffer();t.bindFramebuffer(t.FRAMEBUFFER,u),r.target===t.TEXTURE_2D_ARRAY?t.framebufferTextureLayer(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,r.tex,0,i):t.framebufferTexture2D(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,t.TEXTURE_2D,r.tex,0),t.viewport(a,o,s,c),t.disable(t.BLEND),t.disable(t.DEPTH_TEST),t.disable(t.CULL_FACE),t.disable(t.SCISSOR_TEST),t.disable(t.DITHER),t.useProgram(l);let d=e.target===t.TEXTURE_2D_ARRAY;return d&&t.uniform1i(j.uLayer,n),t.uniform2i(j.uAt[+!d],a,o),t.activeTexture(t.TEXTURE0),t.bindTexture(e.target,e.tex),t.bindVertexArray(j.vao),t.drawArrays(t.TRIANGLES,0,3),t.bindVertexArray(null),t.bindTexture(e.target,null),t.useProgram(null),t.enable(t.DITHER),t.bindFramebuffer(t.FRAMEBUFFER,null),t.deleteFramebuffer(u),!0},I=()=>!s&&t.getError()===t.NO_ERROR&&!t.isContextLost(),L=(e,n)=>{let r=t.createBuffer();t.bindBuffer(t.PIXEL_PACK_BUFFER,r),t.bufferData(t.PIXEL_PACK_BUFFER,e*n*4,t.STREAM_READ),t.readPixels(0,0,e,n,t.RGBA,t.UNSIGNED_BYTE,0),t.bindBuffer(t.PIXEL_PACK_BUFFER,null);let i=t.fenceSync(t.SYNC_GPU_COMMANDS_COMPLETE,0);return t.flush(),new Promise((a,o)=>{let c=()=>{if(s||t.isContextLost()){o(new K(`E-GPU-004`,`the WebGL2 context was lost`,{fix:`reload the page`}));return}if(t.clientWaitSync(i,0,0)===t.TIMEOUT_EXPIRED){setTimeout(c,1);return}t.deleteSync(i);let l=new Uint8Array(e*n*4),u=new Uint8Array(e*n*4);t.bindBuffer(t.PIXEL_PACK_BUFFER,r),t.getBufferSubData(t.PIXEL_PACK_BUFFER,0,l),t.bindBuffer(t.PIXEL_PACK_BUFFER,null),t.deleteBuffer(r);for(let t=0;t<n;t++)u.set(l.subarray((n-1-t)*e*4,(n-t)*e*4),t*e*4);a(u)};c()})},R=/* @__PURE__ */ new Set,z=!1,B=[],V=e=>{z?B.push(e):e()},H=0,U=/* @__PURE__ */ new Set,ee=(e,n,r)=>{let i=e.desc;if(!i.instance&&!i.vertex)return e.vao;let a=(n&&i.instance?n.vid:0)*4194304+(r&&i.vertex?r.vid:0),o=e.vaos.get(a);if(o)return o;o=t.createVertexArray(),t.bindVertexArray(o);let s=(e,n,r)=>{if(n){for(let e of n.attributes)t.enableVertexAttribArray(e.location),t.vertexAttribDivisor(e.location,r);if(e){t.bindBuffer(t.ARRAY_BUFFER,e.buf);for(let e of n.attributes)t.vertexAttribPointer(e.location,e.size,t.FLOAT,!1,n.stride*4,e.offset*4);t.bindBuffer(t.ARRAY_BUFFER,null)}}};return s(n,i.instance,1),s(r,i.vertex,0),e.vaos.set(a,o),o},W=e=>{for(let n of U)for(let[r,i]of n.vaos)(Math.floor(r/4194304)===e||r%4194304===e)&&(t.deleteVertexArray(i),n.vaos.delete(r))},te=()=>{},ne=new Promise(e=>{te=e}),re=e=>{e.preventDefault(),s=!0,te(`the WebGL2 context was lost`)};e.addEventListener(`webglcontextlost`,re),t.pixelStorei(t.UNPACK_ALIGNMENT,1),Ki.open();let ie=!1;t.pixelStorei(t.PACK_ALIGNMENT,1);let ae=t.getExtension(`KHR_parallel_shader_compile`),oe=(e,n)=>{let r=t.createShader(e);return t.shaderSource(r,n),t.compileShader(r),r},se=(e,n,r)=>{if(!t.getShaderParameter(e,t.COMPILE_STATUS)&&!t.isContextLost()){let i=t.getShaderInfoLog(e)??``;throw new K(`E-GPU-003`,`WebGL2 could not compile the ${n===t.VERTEX_SHADER?`vertex`:`fragment`} shader of '${r}': ${i.trim()}`,{where:r,fix:`if this is your shader code, check the line given; otherwise report it with the browser and device`,details:{glsl:t.getShaderSource(e)??``}})}};return{api:`webgl2`,caps:r,lost:ne,stats:u,flush(){s||t.flush()},get width(){return i},get height(){return a},get bytes(){return o},memory:()=>l.list(),resize(t,n){i=Math.max(1,Math.round(t)),a=Math.max(1,Math.round(n)),e.width!==i&&(e.width=i),e.height!==a&&(e.height=a)},buffer(e,n,r){let i=e===`uniform`?t.UNIFORM_BUFFER:e===`index`?t.ELEMENT_ARRAY_BUFFER:t.ARRAY_BUFFER,a=typeof n==`number`?null:n,u=Math.max(e===`uniform`?16:4,Math.ceil((a?a.byteLength:n)/4)*4),f=t.createBuffer();t.bindVertexArray(null),t.bindBuffer(i,f),t.bufferData(i,u,t.DYNAMIC_DRAW),t.bindBuffer(i,null),o+=u;let p=r??e;l.add(p,u);let m={kind:e,size:u,buf:f,target:i,vid:++H,uid:H,version:0,write(e,n=0){m.version++,!s&&(t.bindVertexArray(null),t.bindBuffer(i,f),n===0&&e.byteLength>=u&&t.bufferData(i,u,t.DYNAMIC_DRAW),t.bufferSubData(i,n,e),t.bindBuffer(i,null),d++,c+=e.byteLength)},destroy(){R.delete(m)&&(o-=u,l.remove(p,u),Ki.remove(`buffers`,u),V(()=>{t.deleteBuffer(f),W(m.vid)}))}};return R.add(m),Ki.add(`buffers`,u),a&&m.write(a),m},texture(e){let n=e.layers===void 0?t.TEXTURE_2D:t.TEXTURE_2D_ARRAY,r=e.format===`rgba8`?t.RGBA8:e.format===`rgba32f`?t.RGBA32F:t.R32F;n===t.TEXTURE_2D_ARRAY&&!s&&(M(`layer`),e.format===`rgba8`&&M(`plane`));let i=t.createTexture();t.bindTexture(n,i),n===t.TEXTURE_2D_ARRAY?t.texStorage3D(n,1,r,e.width,e.height,e.layers):t.texStorage2D(n,1,r,e.width,e.height);let a=e.format===`rgba8`&&(e.filter??`linear`)===`linear`?t.LINEAR:t.NEAREST;t.texParameteri(n,t.TEXTURE_MIN_FILTER,a),t.texParameteri(n,t.TEXTURE_MAG_FILTER,a),t.texParameteri(n,t.TEXTURE_WRAP_S,t.CLAMP_TO_EDGE),t.texParameteri(n,t.TEXTURE_WRAP_T,t.CLAMP_TO_EDGE),t.bindTexture(n,null);let u=e.width*e.height*(e.layers??1)*(e.format===`rgba32f`?16:4);o+=u;let f=e.label??`texture`;l.add(f,u);let p=e.format===`r32f`?t.RED:t.RGBA,m=e.format===`rgba8`?t.UNSIGNED_BYTE:t.FLOAT,h={desc:e,tex:i,target:n,write(r,a={}){if(s)return;let o=a.width??e.width,l=a.height??e.height;y(()=>{t.bindTexture(n,i),n===t.TEXTURE_2D_ARRAY?t.texSubImage3D(n,0,a.x??0,a.y??0,a.layer??0,o,l,1,p,m,r):t.texSubImage2D(n,0,a.x??0,a.y??0,o,l,p,m,r),t.bindTexture(n,null)}),d++,c+=o*l*(e.format===`rgba32f`?16:4)},writeImage(r,a={}){s||(a.premultiplied&&t.pixelStorei(t.UNPACK_PREMULTIPLY_ALPHA_WEBGL,!0),y(()=>{if(n===t.TEXTURE_2D_ARRAY){let n=A(r.width,r.height);if(P(),t.bindTexture(t.TEXTURE_2D,n.tex),t.texSubImage2D(t.TEXTURE_2D,0,0,0,r.width,r.height,t.RGBA,t.UNSIGNED_BYTE,r),t.bindTexture(t.TEXTURE_2D,null),!F({tex:n.tex,target:t.TEXTURE_2D},0,h,a.layer??0,a.x??0,a.y??0,r.width,r.height)||!I())throw a.premultiplied&&t.pixelStorei(t.UNPACK_PREMULTIPLY_ALPHA_WEBGL,!1),new K(`E-GPU-005`,`WebGL2 did not draw an image into layer ${a.layer??0} of '${e.label??`texture`}'`,{where:e.label,fix:`nothing to do: it is loaded again`})}else t.bindTexture(n,i),t.texSubImage2D(n,0,a.x??0,a.y??0,r.width,r.height,t.RGBA,t.UNSIGNED_BYTE,r);t.bindTexture(n,null)}),a.premultiplied&&t.pixelStorei(t.UNPACK_PREMULTIPLY_ALPHA_WEBGL,!1),d++,c+=r.width*r.height*4)},destroy(){R.delete(h)&&(o-=u,l.remove(f,u),Ki.remove(`textures`,u),V(()=>t.deleteTexture(i)))}};return R.add(h),Ki.add(`textures`,u),h},copyLayers(e,t,n){let i=e,a=t,o=Math.min(n,i.desc.layers??1,a.desc.layers??1);if(s||o<=0)return!0;if(i.desc.format!==`rgba8`&&!r.floatTargets)return!1;let c=Math.min(i.desc.width,a.desc.width),l=Math.min(i.desc.height,a.desc.height);P();for(let e=0;e<o;e++)if(!F(i,e,a,e,0,0,c,l))return!1;return I()},layerTarget(e,n){let r=e,i=t.createFramebuffer();t.bindFramebuffer(t.FRAMEBUFFER,i),r.target===t.TEXTURE_2D?t.framebufferTexture2D(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,t.TEXTURE_2D,r.tex,0):t.framebufferTextureLayer(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,r.tex,0,n),t.bindFramebuffer(t.FRAMEBUFFER,null);let a={width:r.desc.width,height:r.desc.height,scene:null,layerFb:i,destroy(){R.delete(a)&&(Ki.remove(`targets`),V(()=>t.deleteFramebuffer(i)))}};return R.add(a),Ki.add(`targets`),a},target(e,n,r={}){let i=Math.max(1,Math.round(e)),a=Math.max(1,Math.round(n)),s=b(i,a,!!r.sampled);o+=i*a*8;let c=r.label??`target`;l.add(c,i*a*8);let u={width:i,height:a,scene:s,texture:s.colorTex?{desc:{label:`target`,width:i,height:a,format:`rgba8`,filter:`nearest`},tex:s.colorTex,target:t.TEXTURE_2D,write(){throw new K(`E-GPU-003`,`a target is drawn into, not written`,{fix:`draw into it with frame({ target })`})},writeImage(){throw new K(`E-GPU-003`,`a target is drawn into, not written`,{fix:`draw into it with frame({ target })`})},destroy(){}}:void 0,destroy(){R.delete(u)&&(o-=i*a*8,l.remove(c,i*a*8),Ki.remove(`targets`,i*a*8),V(()=>x(s)))}};return R.add(u),Ki.add(`targets`,i*a*8),u},program(e){let n={label:e.label,desc:e,prog:null,vao:null,ok:!1,depthUnit:(e.textures??[]).length,groundUnit:(e.textures??[]).length+1,vaos:/* @__PURE__ */ new Map,ready:Promise.resolve(),destroy(){if(R.delete(n)){Ki.remove(`programs`),n.prog&&t.deleteProgram(n.prog),n.vao&&t.deleteVertexArray(n.vao);for(let e of n.vaos.values())t.deleteVertexArray(e);n.vaos.clear(),U.delete(n)}}},r=oe(t.VERTEX_SHADER,e.glsl.vertex),i=oe(t.FRAGMENT_SHADER,e.glsl.fragment),a=t.createProgram();t.attachShader(a,r),t.attachShader(a,i),t.linkProgram(a);let o=()=>{if(!t.getProgramParameter(a,t.LINK_STATUS)&&!t.isContextLost()){try{se(r,t.VERTEX_SHADER,e.label),se(i,t.FRAGMENT_SHADER,e.label)}finally{t.deleteShader(r),t.deleteShader(i)}let n=t.getProgramInfoLog(a)??``;throw t.deleteProgram(a),new K(`E-GPU-003`,`WebGL2 could not link '${e.label}': ${n.trim()}`,{where:e.label,fix:`the vertex and fragment shaders must agree on their in / out values`})}if(t.deleteShader(r),t.deleteShader(i),e.blocks.forEach((e,n)=>{let r=t.getUniformBlockIndex(a,e);r!==t.INVALID_INDEX&&t.uniformBlockBinding(a,r,n)}),t.useProgram(a),(e.textures??[]).forEach((e,n)=>{let r=t.getUniformLocation(a,e.name);r&&t.uniform1i(r,n)}),e.sceneDepth){let e=t.getUniformLocation(a,`sceneDepth`);e&&t.uniform1i(e,n.depthUnit)}if(e.groundDepth){let e=t.getUniformLocation(a,`groundDepth`);e&&t.uniform1i(e,n.groundUnit)}t.useProgram(null);let o=t.createVertexArray();t.bindVertexArray(o);for(let n of e.instance?.attributes??[])t.enableVertexAttribArray(n.location),t.vertexAttribDivisor(n.location,1);for(let n of e.vertex?.attributes??[])t.enableVertexAttribArray(n.location),t.vertexAttribDivisor(n.location,0);t.bindVertexArray(null),n.prog=a,n.vao=o,n.ok=!0},s=e=>{n.ready=Promise.reject(e),n.ready.catch(()=>{})};if(R.add(n),Ki.add(`programs`),U.add(n),ae&&!t.isContextLost())n.ready=new Promise((e,s)=>{let c=()=>{if(R.has(n)&&!t.isContextLost()&&!t.getProgramParameter(a,ae.COMPLETION_STATUS_KHR))setTimeout(c,4);else try{R.has(n)?o():(t.deleteProgram(a),t.deleteShader(r),t.deleteShader(i)),e()}catch(e){s(e)}};c()}),n.ready.catch(()=>{});else try{o()}catch(e){s(e)}return n},readTexture(e){if(s||t.isContextLost())return Promise.reject(new K(`E-GPU-004`,`the WebGL2 context was lost`,{fix:`reload the page`}));let n=e,r=n.desc,i=r.width,a=r.height,o=r.format!==`rgba8`;if(!r.renderable&&!r.growable)return Promise.reject(Ji(r));let c=t.createFramebuffer();t.bindFramebuffer(t.FRAMEBUFFER,c),n.target===t.TEXTURE_2D?t.framebufferTexture2D(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,t.TEXTURE_2D,n.tex,0):t.framebufferTextureLayer(t.FRAMEBUFFER,t.COLOR_ATTACHMENT0,n.tex,0,0);let l=i*a*4*(o?4:1),u=t.createBuffer();t.bindBuffer(t.PIXEL_PACK_BUFFER,u),t.bufferData(t.PIXEL_PACK_BUFFER,l,t.STREAM_READ),t.readPixels(0,0,i,a,t.RGBA,o?t.FLOAT:t.UNSIGNED_BYTE,0),t.bindBuffer(t.PIXEL_PACK_BUFFER,null),t.bindFramebuffer(t.FRAMEBUFFER,null),t.deleteFramebuffer(c);let d=t.fenceSync(t.SYNC_GPU_COMMANDS_COMPLETE,0);return t.flush(),new Promise((e,n)=>{let c=()=>{if(s||t.isContextLost()){n(new K(`E-GPU-004`,`the WebGL2 context was lost`,{fix:`reload the page`}));return}if(t.clientWaitSync(d,0,0)===t.TIMEOUT_EXPIRED){setTimeout(c,1);return}t.deleteSync(d);let l=o?new Float32Array(i*a*4):new Uint8Array(i*a*4);if(t.bindBuffer(t.PIXEL_PACK_BUFFER,u),t.getBufferSubData(t.PIXEL_PACK_BUFFER,0,l),t.bindBuffer(t.PIXEL_PACK_BUFFER,null),t.deleteBuffer(u),r.format!==`r32f`){e(l);return}let f=new Float32Array(i*a);for(let e=0;e<f.length;e++)f[e]=l[e*4];e(f)};c()})},frame(e,n){if(s||t.isContextLost())return n.read?Promise.reject(new K(`E-GPU-004`,`the WebGL2 context was lost`,{fix:`reload the page`})):null;let r=n.target,i=r?r.width:this.width,a=r?r.height:this.height;if(r?.layerFb){t.bindFramebuffer(t.FRAMEBUFFER,r.layerFb),t.viewport(0,0,i,a),t.clearColor(...n.clear),t.clear(t.COLOR_BUFFER_BIT),t.disable(t.DEPTH_TEST),t.depthMask(!1),t.disable(t.CULL_FACE);for(let n of e){let e=n.program;if(!e.ok||!e.prog)continue;t.useProgram(e.prog);let r=e.desc.format&&e.desc.format!==`rgba8`?`none`:e.desc.blend??`none`;r===`none`?t.disable(t.BLEND):(t.enable(t.BLEND),t.blendFuncSeparate(r===`premultiplied`?t.ONE:t.SRC_ALPHA,t.ONE_MINUS_SRC_ALPHA,t.ONE,t.ONE_MINUS_SRC_ALPHA)),t.bindVertexArray(e.vao);for(let e=0;e<n.blocks.length;e++)t.bindBufferBase(t.UNIFORM_BUFFER,e,n.blocks[e].buf);let i=n.textures;if(i)for(let e=0;e<i.length;e++)t.activeTexture(t.TEXTURE0+e),t.bindTexture(i[e].target,i[e].tex);t.drawArraysInstanced(e.desc.topology===`lines`?t.LINES:t.TRIANGLES,n.first??0,n.count,n.instanceCount??1),t.bindVertexArray(null)}return t.depthMask(!0),t.bindFramebuffer(t.FRAMEBUFFER,null),null}let o=e.some(e=>e.program.ok&&e.program.desc.sceneDepth||e.keepDepth),l=r?r.scene:null,p=!!n.aa&&!r&&!n.keep&&!n.scissor&&!!O();p?((!T||T.w!==i||T.h!==a)&&(T&&x(T),T=b(i,a,!0),t.bindTexture(t.TEXTURE_2D,T.colorTex),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MIN_FILTER,t.LINEAR),t.texParameteri(t.TEXTURE_2D,t.TEXTURE_MAG_FILTER,t.LINEAR),t.bindTexture(t.TEXTURE_2D,null)),l=T):!r&&o&&((!w||w.w!==i||w.h!==a)&&(w&&x(w),w=b(i,a)),l=w),r||(v(),g()),t.bindFramebuffer(t.FRAMEBUFFER,l?l.fb:null),t.viewport(0,0,i,a),n.scissor&&(t.enable(t.SCISSOR_TEST),t.scissor(Math.floor(n.scissor[0]),Math.floor(a-n.scissor[1]-n.scissor[3]),Math.ceil(n.scissor[2]),Math.ceil(n.scissor[3]))),t.clearColor(...n.clear),t.clearDepth(1),t.depthMask(!0);let m=(n.keep&&!n.scissor?0:t.COLOR_BUFFER_BIT)|(n.keep&&n.keepDepth?0:t.DEPTH_BUFFER_BIT);m&&t.clear(m);let h=!0,y=0,k=1,A=null,j=``,M=``,N=``,P=[],F=[],I=!1;for(let n of e){n.keepDepth&&l&&!r&&!I&&(C(l),I=!0,k++,F.length=0);let e=n.program;if(!e.ok||!e.prog)continue;let i=e.desc;if(i.sceneDepth&&l)h&&(S(l),h=!1,k++,F.length=0);else if(i.sceneDepth)continue;A!==e.prog&&(t.useProgram(e.prog),A=e.prog);let a=i.blend??`none`;a===j||(a===`none`?(t.disable(t.BLEND),j=a):(j=a,t.enable(t.BLEND),a===`alpha`?t.blendFuncSeparate(t.SRC_ALPHA,t.ONE_MINUS_SRC_ALPHA,t.ONE,t.ONE_MINUS_SRC_ALPHA):a===`add`?t.blendFuncSeparate(t.SRC_ALPHA,t.ONE,t.ONE,t.ONE):a===`multiply`?t.blendFuncSeparate(t.DST_COLOR,t.ONE_MINUS_SRC_ALPHA,t.ONE,t.ONE_MINUS_SRC_ALPHA):a===`screen`?t.blendFuncSeparate(t.ONE_MINUS_DST_COLOR,t.ONE,t.ONE,t.ONE_MINUS_SRC_ALPHA):t.blendFuncSeparate(t.ONE,t.ONE_MINUS_SRC_ALPHA,t.ONE,t.ONE_MINUS_SRC_ALPHA)));let o=i.depth?.test??`less`,s=i.sceneDepth?!1:i.depth?.write??!0,c=`${o}${s}`;c!==M&&(t.enable(t.DEPTH_TEST),t.depthFunc(o===`less`?t.LESS:o===`less-equal`?t.LEQUAL:o===`greater-equal`?t.GEQUAL:t.ALWAYS),t.depthMask(s),M=c),s&&(h=!0);let u=i.cull??`none`;u!==N&&(u===`none`?t.disable(t.CULL_FACE):(t.enable(t.CULL_FACE),t.cullFace(u===`back`?t.BACK:t.FRONT)),N=u),t.bindVertexArray(ee(e,n.instances,n.vertices));for(let e=0;e<n.blocks.length;e++){let r=n.blocks[e].buf;P[e]!==r&&(t.bindBufferBase(t.UNIFORM_BUFFER,e,r),P[e]=r)}let d=n.textures;if(d)for(let e=0;e<d.length;e++){let n=d[e].tex;F[e]!==n&&(t.activeTexture(t.TEXTURE0+e),t.bindTexture(d[e].target,n),F[e]=n)}if(i.sceneDepth&&l&&(t.activeTexture(t.TEXTURE0+e.depthUnit),t.bindTexture(t.TEXTURE_2D,l.copy),F[e.depthUnit]=l.copy),i.groundDepth&&l){let n=I&&l.kept?l.kept:l.copy;t.activeTexture(t.TEXTURE0+e.groundUnit),t.bindTexture(t.TEXTURE_2D,n),F[e.groundUnit]=n}n.index?(t.bindBuffer(t.ELEMENT_ARRAY_BUFFER,n.index.buf),t.drawElementsInstanced(i.topology===`lines`?t.LINES:t.TRIANGLES,n.count,t.UNSIGNED_INT,0,n.instanceCount??1)):t.drawArraysInstanced(i.topology===`lines`?t.LINES:t.TRIANGLES,n.first??0,n.count,n.instanceCount??1),y++}if(n.scissor&&t.disable(t.SCISSOR_TEST),t.bindVertexArray(null),p&&l){t.bindFramebuffer(t.FRAMEBUFFER,null),t.viewport(0,0,i,a),t.disable(t.DEPTH_TEST),t.depthMask(!1),t.disable(t.BLEND),t.disable(t.CULL_FACE),t.useProgram(E),t.activeTexture(t.TEXTURE0),t.bindTexture(t.TEXTURE_2D,l.colorTex),t.bindVertexArray(D),t.drawArrays(t.TRIANGLES,0,3),t.bindVertexArray(null),t.depthMask(!0),k++;let e=n.read?L(i,a):null;return r||(_(),u.draws=y,u.passes=k,u.uploadBytes=c,c=0,u.uploads=d,d=0,u.uploadMaxMs=+f.toFixed(2),f=0),e}let R=n.read?L(i,a):null;return!r&&l&&(t.bindFramebuffer(t.READ_FRAMEBUFFER,l.fb),t.bindFramebuffer(t.DRAW_FRAMEBUFFER,null),t.blitFramebuffer(0,0,i,a,0,0,i,a,t.COLOR_BUFFER_BIT,t.NEAREST),k++),t.bindFramebuffer(t.FRAMEBUFFER,null),r||(_(),u.draws=y,u.passes=k,u.uploadBytes=c,c=0,u.uploads=d,d=0,u.uploadMaxMs=+f.toFixed(2),f=0),R},hold(e){if(z=e,!e&&B.length)for(let e of B.splice(0))e()},destroy(){if(!ie){ie=!0,z=!1;for(let e of[...R])e.destroy();for(let e of B.splice(0))e();w&&=(x(w),null),T&&=(x(T),null),E&&=(t.deleteProgram(E),null),k&&=(t.deleteTexture(k.tex),null);for(let e of[j.layer,j.plane,...Object.values(j.linking)])e&&t.deleteProgram(e);j.vao&&t.deleteVertexArray(j.vao),j.layer=j.plane=j.vao=null,j.linking={},_(),m&&=(t.deleteQuery(m),null),Ki.close(),e.removeEventListener(`webglcontextlost`,re),t.getExtension(`WEBGL_lose_context`)?.loseContext(),s=!0}}}}const Qi={rgba8:`rgba8unorm`,rgba32f:`rgba32float`,r32f:`r32float`},$i={rgba8:4,rgba32f:16,r32f:4},ea={1:`float32`,2:`float32x2`,3:`float32x3`,4:`float32x4`},ta={none:void 0,alpha:{color:{srcFactor:`src-alpha`,dstFactor:`one-minus-src-alpha`},alpha:{srcFactor:`one`,dstFactor:`one-minus-src-alpha`}},add:{color:{srcFactor:`src-alpha`,dstFactor:`one`},alpha:{srcFactor:`one`,dstFactor:`one`}},premultiplied:{color:{srcFactor:`one`,dstFactor:`one-minus-src-alpha`},alpha:{srcFactor:`one`,dstFactor:`one-minus-src-alpha`}},multiply:{color:{srcFactor:`dst`,dstFactor:`one-minus-src-alpha`},alpha:{srcFactor:`one`,dstFactor:`one-minus-src-alpha`}},screen:{color:{srcFactor:`one-minus-dst`,dstFactor:`one`},alpha:{srcFactor:`one`,dstFactor:`one-minus-src-alpha`}}},na={less:`less`,"less-equal":`less-equal`,"greater-equal":`greater-equal`,always:`always`};let ra=1;async function ia(e,t={}){let n=navigator.gpu;if(!n)throw Error(`navigator.gpu is missing`);let r=await n.requestAdapter({powerPreference:t.powerPreference??`high-performance`});if(!r)throw Error(`no WebGPU adapter (the browser has WebGPU, but no usable graphics chip for it)`);let i=await r.requestDevice({requiredLimits:{maxTextureArrayLayers:Math.min(256,r.limits.maxTextureArrayLayers)}}),a=e.getContext(`webgpu`);if(!a)throw Error(`the canvas gave no WebGPU context (was it used for WebGL before?)`);let o=n.getPreferredCanvasFormat(),s=!1,c=()=>a.configure({device:i,format:o,alphaMode:`opaque`,usage:GPUTextureUsage.RENDER_ATTACHMENT|(s?GPUTextureUsage.COPY_SRC:0)});c();let l=r.info,u={api:`webgpu`,compute:!0,timer:r.features.has(`timestamp-query`),floatTargets:!0,maxTex:i.limits.maxTextureDimension2D,maxLayers:i.limits.maxTextureArrayLayers,vertexTextures:16,f16:r.features.has(`shader-f16`),device:[l?.vendor,l?.architecture,l?.description].filter(Boolean).join(` `)},d=e.width,f=e.height,p=null,m=0,h=!1,g=0,_=new qi,v={draws:0,passes:0,uploadBytes:0,uploads:0,gpuMs:null,uploadMaxMs:0},y=0,b=GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.TEXTURE_BINDING,x=/* @__PURE__ */ new WeakMap,S=(e,t)=>{let n=x.get(e);n||(n=/* @__PURE__ */ new Map,x.set(e,n));let r=n.get(t);return r||(r=i.createBindGroup({layout:t,entries:[{binding:0,resource:e.createView()}]}),n.set(t,r)),r},C=0,w=/* @__PURE__ */ new Set,T=/* @__PURE__ */ new Map,E=null,D=!1,O=[],k=e=>{let t=[];if(E&&=(t.push(E.finish()),null),e&&t.push(e.finish()),t.length&&i.queue.submit(t),O.length&&!D)for(let e of O.splice(0))e()},A=e=>{E||D?O.push(e):e()},j=(e,t,n)=>{let r=`${e}${t}`;for(let e=0;e<n.length;e++)r+=`${n[e].id},`;return r},M=i.createSampler({magFilter:`linear`,minFilter:`linear`,addressModeU:`clamp-to-edge`,addressModeV:`clamp-to-edge`}),N=()=>{p&&p.width===d&&p.height===f||(p?.destroy(),p=i.createTexture({size:[Math.max(1,d),Math.max(1,f)],format:`depth24plus`,usage:b}))},P=null,F=null,I=null,L=null,R=()=>{if(F)return!0;if(!I){let e=i.createShaderModule({label:`fxaa`,code:`
@group(0) @binding(0) var pic: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;
struct VOut { @builtin(position) pos: vec4f };
@vertex fn vs(@builtin(vertex_index) i: u32) -> VOut {
  var o: VOut;
  let x = f32((i << 1u) & 2u) * 2.0 - 1.0;
  let y = f32(i & 2u) * 2.0 - 1.0;
  o.pos = vec4f(x, y, 0.0, 1.0);
  return o;
}
fn luma(c: vec3f) -> f32 {
  return dot(c, vec3f(0.299, 0.587, 0.114));
}
fn at(uv: vec2f) -> vec3f {
  return textureSampleLevel(pic, smp, uv, 0.0).rgb;
}
fn fxaa(uv: vec2f, px: vec2f) -> vec3f {
  let nw = luma(at(uv + vec2f(-1.0, -1.0) * px));
  let ne = luma(at(uv + vec2f(1.0, -1.0) * px));
  let sw = luma(at(uv + vec2f(-1.0, 1.0) * px));
  let se = luma(at(uv + vec2f(1.0, 1.0) * px));
  let m = luma(at(uv));
  let lo = min(m, min(min(nw, ne), min(sw, se)));
  let hi = max(m, max(max(nw, ne), max(sw, se)));
  var dir = vec2f(-((nw + ne) - (sw + se)), (nw + sw) - (ne + se));
  let reduce = max((nw + ne + sw + se) * 0.03125, 0.0078125);
  let rcp = 1.0 / (min(abs(dir.x), abs(dir.y)) + reduce);
  dir = clamp(dir * rcp, vec2f(-8.0), vec2f(8.0)) * px;
  let a = 0.5 * (at(uv + dir * (1.0 / 3.0 - 0.5)) + at(uv + dir * (2.0 / 3.0 - 0.5)));
  let b = a * 0.5 + 0.25 * (at(uv - dir * 0.5) + at(uv + dir * 0.5));
  let lb = luma(b);
  if (lb < lo || lb > hi) {
    return a;
  }
  return b;
}
@fragment fn fs(v: VOut) -> @location(0) vec4f {
  let size = vec2f(textureDimensions(pic));
  return vec4f(fxaa(v.pos.xy / size, 1.0 / size), 1.0);
}
`});I=i.createRenderPipelineAsync({label:`fxaa`,layout:`auto`,vertex:{module:e,entryPoint:`vs`},fragment:{module:e,entryPoint:`fs`,targets:[{format:o}]},primitive:{topology:`triangle-list`}}).then(e=>{F=e},()=>{})}return!1},z=()=>{P&&P.width===d&&P.height===f||(P?.destroy(),P=i.createTexture({label:`fxaa picture`,size:[Math.max(1,d),Math.max(1,f)],format:o,usage:GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.TEXTURE_BINDING}),L=i.createBindGroup({layout:F.getBindGroupLayout(0),entries:[{binding:0,resource:P.createView()},{binding:1,resource:M}]}))},B=null,V=null,H=null,U=null,ee=/* @__PURE__ */ new WeakMap,W=/* @__PURE__ */ new WeakMap,te=()=>{if(H)return!0;if(!U){let e=i.createShaderModule({label:`kept depth`,code:`@group(0) @binding(0) var d: texture_depth_2d;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) p: vec4f) -> @location(0) vec4f {
  return vec4f(textureLoad(d, vec2i(p.xy), 0), 0.0, 0.0, 1.0);
}`});U=i.createRenderPipelineAsync({label:`kept depth`,layout:`auto`,vertex:{module:e,entryPoint:`vs`},fragment:{module:e,entryPoint:`fs`,targets:[{format:`r32float`}]},primitive:{topology:`triangle-list`}}).then(e=>{H=e},()=>{})}return!1},ne=()=>{if(!(B&&B.width===d&&B.height===f)){if(B){m-=B.width*B.height*4,_.remove(`kept depth`,B.width*B.height*4);let e=B;A(()=>e.destroy())}B=i.createTexture({label:`kept depth`,size:[Math.max(1,d),Math.max(1,f)],format:`r32float`,usage:GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.TEXTURE_BINDING}),V=B.createView(),m+=B.width*B.height*4,_.add(`kept depth`,B.width*B.height*4)}},re=i.lost.then(e=>(h=!0,e.message||String(e.reason)));Ki.open();let ie=!1;return{api:`webgpu`,caps:u,lost:re,stats:v,flush(){h||k(null)},get width(){return d},get height(){return f},get bytes(){return m},memory:()=>_.list(),resize(t,n){d=Math.max(1,Math.round(t)),f=Math.max(1,Math.round(n)),e.width!==d&&(e.width=d),e.height!==f&&(e.height=f)},buffer(e,t,n){let r=typeof t==`number`?null:t,a=Math.max(e===`uniform`?16:4,Math.ceil((r?r.byteLength:t)/4)*4),o=(e===`uniform`?GPUBufferUsage.UNIFORM:e===`index`?GPUBufferUsage.INDEX:GPUBufferUsage.VERTEX)|GPUBufferUsage.COPY_DST,s=i.createBuffer({size:a,usage:o,label:n});m+=a;let c=n??e;_.add(c,a);let l={id:ra++,kind:e,size:a,buf:s,uid:++C,version:0,write(e,t=0){h||(i.queue.writeBuffer(s,t,e.buffer,e.byteOffset,e.byteLength),y++,g+=e.byteLength,l.version++)},destroy(){w.delete(l)&&(A(()=>s.destroy()),m-=a,_.remove(c,a),T.clear(),Ki.remove(`buffers`,a))}};return w.add(l),Ki.add(`buffers`,a),r&&l.write(r),l},texture(e){let t=i.createTexture({label:e.label,size:[e.width,e.height,e.layers??1],format:Qi[e.format],usage:GPUTextureUsage.TEXTURE_BINDING|GPUTextureUsage.COPY_DST|(e.format===`rgba8`||e.renderable?GPUTextureUsage.RENDER_ATTACHMENT:0)|(e.renderable||e.growable?GPUTextureUsage.COPY_SRC:0)}),n=e.width*e.height*(e.layers??1)*$i[e.format];m+=n;let r=e.label??`texture`;_.add(r,n);let a={id:ra++,desc:e,tex:t,view:t.createView({dimension:e.layers===void 0?`2d`:`2d-array`}),write(n,r={}){if(h)return;let a=r.width??e.width,o=r.height??e.height;i.queue.writeTexture({texture:t,origin:[r.x??0,r.y??0,r.layer??0]},n.buffer,{offset:n.byteOffset,bytesPerRow:a*$i[e.format],rowsPerImage:o},[a,o,1]),y++,g+=a*o*$i[e.format]},writeImage(e,n={}){h||(i.queue.copyExternalImageToTexture({source:e},{texture:t,origin:[n.x??0,n.y??0,n.layer??0],premultipliedAlpha:!!n.premultiplied},[e.width,e.height,1]),y++,g+=e.width*e.height*4)},destroy(){w.delete(a)&&(A(()=>t.destroy()),m-=n,_.remove(r,n),T.clear(),Ki.remove(`textures`,n))}};return w.add(a),Ki.add(`textures`,n),a},copyLayers(e,t,n){let r=e,a=t,o=Math.min(n,r.desc.layers??1,a.desc.layers??1);if(h||o<=0)return!0;let s=i.createCommandEncoder({label:`copy layers`});return s.copyTextureToTexture({texture:r.tex},{texture:a.tex},[Math.min(r.desc.width,a.desc.width),Math.min(r.desc.height,a.desc.height),o]),i.queue.submit([s.finish()]),!0},layerTarget(e,t){let n=e,r={width:n.desc.width,height:n.desc.height,layerView:n.tex.createView({dimension:`2d`,baseArrayLayer:t,arrayLayerCount:1}),destroy(){w.delete(r)&&Ki.remove(`targets`)}};return w.add(r),Ki.add(`targets`),r},target(e,t,n={}){let r=Math.max(1,Math.round(e)),a=Math.max(1,Math.round(t)),s=i.createTexture({size:[r,a],format:o,usage:GPUTextureUsage.RENDER_ATTACHMENT|GPUTextureUsage.COPY_SRC|(n.sampled?GPUTextureUsage.TEXTURE_BINDING:0)}),c=i.createTexture({size:[r,a],format:`depth24plus`,usage:b});m+=r*a*8;let l=n.label??`target`;_.add(l,r*a*8);let u={width:r,height:a,color:s,depth:c,texture:n.sampled?{id:ra++,desc:{label:`target`,width:r,height:a,format:`rgba8`,filter:`nearest`},tex:s,view:s.createView(),write(){throw new K(`E-GPU-003`,`a target is drawn into, not written`,{fix:`draw into it with frame({ target })`})},writeImage(){throw new K(`E-GPU-003`,`a target is drawn into, not written`,{fix:`draw into it with frame({ target })`})},destroy(){}}:void 0,destroy(){w.delete(u)&&(A(()=>{s.destroy(),c.destroy()}),m-=r*a*8,_.remove(l,r*a*8),Ki.remove(`targets`,r*a*8),T.clear())}};return w.add(u),Ki.add(`targets`,r*a*8),u},program(e){i.pushErrorScope(`validation`);let t=i.createShaderModule({label:e.label,code:e.wgsl}),n=GPUShaderStage.VERTEX|GPUShaderStage.FRAGMENT,r=[i.createBindGroupLayout({label:`${e.label} blocks`,entries:e.blocks.map((e,t)=>({binding:t,visibility:n,buffer:{type:`uniform`}}))})];e.textures?.length&&r.push(i.createBindGroupLayout({label:`${e.label} textures`,entries:[...e.textures.map((e,t)=>({binding:t,visibility:n,texture:{sampleType:e.sample===`float`?`float`:`unfilterable-float`,viewDimension:e.kind}})),{binding:e.textures.length,visibility:GPUShaderStage.FRAGMENT,sampler:{type:`filtering`}}]}));let a=e.sceneDepth?i.createBindGroupLayout({label:`${e.label} scene depth`,entries:[{binding:0,visibility:n,texture:{sampleType:`depth`,viewDimension:`2d`}},...e.groundDepth?[{binding:1,visibility:n,texture:{sampleType:`unfilterable-float`,viewDimension:`2d`}}]:[]]}):null;e.groundDepth&&te(),a&&(r.length<2&&r.push(i.createBindGroupLayout({label:`${e.label} no textures`,entries:[]})),r.push(a));let s=e=>e.map(e=>({shaderLocation:e.location,offset:e.offset*4,format:ea[e.size]})),c={label:e.label,layout:i.createPipelineLayout({bindGroupLayouts:r}),vertex:{module:t,entryPoint:`vs`,buffers:[...e.instance?[{arrayStride:e.instance.stride*4,stepMode:`instance`,attributes:s(e.instance.attributes)}]:[],...e.vertex?[{arrayStride:e.vertex.stride*4,stepMode:`vertex`,attributes:s(e.vertex.attributes)}]:[]]},fragment:{module:t,entryPoint:`fs`,targets:[{format:e.overlay?Qi[e.format??`rgba8`]:o,blend:e.overlay&&e.format&&e.format!==`rgba8`?void 0:ta[e.blend??`none`]}]},primitive:{topology:e.topology===`lines`?`line-list`:`triangle-list`,cullMode:e.cull??`none`},...e.overlay?{}:{depthStencil:{format:`depth24plus`,depthWriteEnabled:e.sceneDepth?!1:e.depth?.write??!0,depthCompare:na[e.depth?.test??`less`]}}},l=i.createRenderPipelineAsync(c).then(e=>e,e=>e),u={id:ra++,label:e.label,desc:e,pipe:null,layouts:r,ok:!1,ready:Promise.resolve(),vslot:+!!e.instance,depthLayout:a,destroy(){w.delete(u)&&Ki.remove(`programs`)}};return u.ready=(async()=>{let[n,r,a]=await Promise.all([i.popErrorScope(),t.getCompilationInfo(),l]),o=r.messages.filter(e=>e.type===`error`).map(e=>`line ${e.lineNum}:${e.linePos} ${e.message}`),s=a instanceof Error?a:null;if(n||o.length||s)throw new K(`E-GPU-003`,`WebGPU could not build '${e.label}': ${o.join(`; `)||n?.message||s?.message}`,{where:e.label,fix:`if this is your shader code, check the line given; otherwise report it with the browser and device`,details:{wgsl:e.wgsl}});w.has(u)&&(u.pipe=a,u.ok=!0)})(),u.ready.catch(()=>{}),w.add(u),Ki.add(`programs`),u},readTexture(e){if(h)return Promise.reject(new K(`E-GPU-004`,`the WebGPU device was lost`,{fix:`reload the page`}));let t=e,n=t.desc,r=$i[n.format],a=n.width,o=n.height,s=Math.ceil(a*r/256)*256;if(!(t.tex.usage&GPUTextureUsage.COPY_SRC))return Promise.reject(Ji(n));E&&k(null);let c=i.createBuffer({size:s*o,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ}),l=i.createCommandEncoder();return l.copyTextureToBuffer({texture:t.tex},{buffer:c,bytesPerRow:s},[a,o,1]),k(l),c.mapAsync(GPUMapMode.READ).then(()=>{let e=new Uint8Array(c.getMappedRange()),t=new Uint8Array(a*o*r);for(let n=0;n<o;n++)t.set(e.subarray(n*s,n*s+a*r),n*a*r);return c.unmap(),c.destroy(),n.format===`rgba8`?t:new Float32Array(t.buffer)})},frame(e,t){if(h)return t.read?Promise.reject(new K(`E-GPU-004`,`the WebGPU device was lost`,{fix:`reload the page`})):null;let n=t.target,r=(e,t,n)=>{let r=j(t.id,`b`,n.blocks),a=T.get(r);if(a||(a=i.createBindGroup({layout:t.layouts[0],entries:n.blocks.map((e,t)=>({binding:t,resource:{buffer:e.buf}}))}),T.set(r,a)),e.setBindGroup(0,a),t.layouts[1]&&n.textures?.length){let r=j(t.id,`t`,n.textures),a=T.get(r);return a||(a=i.createBindGroup({layout:t.layouts[1],entries:[...n.textures.map((e,t)=>({binding:t,resource:e.view})),{binding:n.textures.length,resource:M}]}),T.set(r,a)),e.setBindGroup(1,a),!0}return!1};if(n?.layerView){E||(E=i.createCommandEncoder(),queueMicrotask(()=>{E&&!h&&k(null)}));let a=E.beginRenderPass({colorAttachments:[{view:n.layerView,clearValue:{r:t.clear[0],g:t.clear[1],b:t.clear[2],a:t.clear[3]},loadOp:`clear`,storeOp:`store`}]});for(let t of e){let e=t.program;e.ok&&(a.setPipeline(e.pipe),r(a,e,t),a.draw(t.count,t.instanceCount??1,t.first??0))}return a.end(),null}n||N(),t.read&&!n&&!s&&(s=!0,c());let l=n?n.color:a.getCurrentTexture(),u=i.createCommandEncoder(),d=n?n.width:this.width,f=n?n.height:this.height,m=!!t.aa&&!n&&!t.keep&&!t.scissor&&R();m&&z();let _=n?n.depth:p,b=m?P.createView():l.createView(),x=_.createView(),C=1,w=!1;for(let t of e){let e=t.program;if(!e.ok)continue;let n=!!e.depthLayout;n!==w&&(C++,w=n)}let D=!n&&e.some(e=>e.keepDepth)&&te();D&&C++;let O=!1,A=!1,I=!0,U=0,re=0,ie=t.scissor?[Math.max(0,Math.floor(t.scissor[0])),Math.max(0,Math.floor(t.scissor[1]))]:null,ae=t.scissor&&ie?[Math.max(0,Math.min(d-ie[0],Math.ceil(t.scissor[2]))),Math.max(0,Math.min(f-ie[1],Math.ceil(t.scissor[3])))]:null,oe=e=>{U++;let n=u.beginRenderPass({colorAttachments:[{view:b,clearValue:{r:t.clear[0],g:t.clear[1],b:t.clear[2],a:t.clear[3]},loadOp:I&&!t.keep?`clear`:`load`,storeOp:`store`}],depthStencilAttachment:e?{view:x,depthReadOnly:!0}:{view:x,depthClearValue:1,depthLoadOp:I&&!(t.keep&&t.keepDepth)?`clear`:`load`,depthStoreOp:U===C&&!t.keepDepth?`discard`:`store`}});return ie&&ae&&n.setScissorRect(ie[0],ie[1],ae[0],ae[1]),I=!1,A=e,n},se=oe(!1);for(let t of e){if(D&&!O&&t.keepDepth){se.end(),ne();let e=ee.get(_);e||(e=i.createBindGroup({layout:H.getBindGroupLayout(0),entries:[{binding:0,resource:_.createView()}]}),ee.set(_,e));let t=u.beginRenderPass({colorAttachments:[{view:V,clearValue:{r:1,g:0,b:0,a:1},loadOp:`clear`,storeOp:`store`}]});t.setPipeline(H),t.setBindGroup(0,e),t.draw(3),t.end(),O=!0,se=oe(A)}let e=t.program;if(!e.ok||e.desc.groundDepth&&!O)continue;let n=!!e.depthLayout;if(n!==A&&(se.end(),se=oe(n)),se.setPipeline(e.pipe),!r(se,e,t)&&e.depthLayout){let t=`${e.id}:none`,n=T.get(t);n||(n=i.createBindGroup({layout:e.layouts[1],entries:[]}),T.set(t,n)),se.setBindGroup(1,n)}if(e.depthLayout&&e.desc.groundDepth){let t=W.get(e.depthLayout);(!t||t.d!==_||t.k!==B)&&(t={d:_,k:B,g:i.createBindGroup({layout:e.depthLayout,entries:[{binding:0,resource:_.createView()},{binding:1,resource:V}]})},W.set(e.depthLayout,t)),se.setBindGroup(2,t.g)}else e.depthLayout&&se.setBindGroup(2,S(_,e.depthLayout));t.instances&&se.setVertexBuffer(0,t.instances.buf),t.vertices&&se.setVertexBuffer(e.vslot,t.vertices.buf),t.index?(se.setIndexBuffer(t.index.buf,`uint32`),se.drawIndexed(t.count,t.instanceCount??1)):se.draw(t.count,t.instanceCount??1,t.first??0),re++}if(n||(v.draws=re,v.passes=U+ +!!O,v.uploadBytes=g,g=0,v.uploads=y,y=0),se.end(),m){let e=u.beginRenderPass({colorAttachments:[{view:l.createView(),clearValue:{r:t.clear[0],g:t.clear[1],b:t.clear[2],a:t.clear[3]},loadOp:`clear`,storeOp:`store`}]});e.setPipeline(F),e.setBindGroup(0,L),e.draw(3),e.end(),n||(v.passes=U+(O?2:1))}if(T.size>1024&&T.clear(),!t.read)return k(u),null;let ce=Math.ceil(d*4/256)*256,le=i.createBuffer({size:ce*f,usage:GPUBufferUsage.COPY_DST|GPUBufferUsage.MAP_READ});u.copyTextureToBuffer({texture:l},{buffer:le,bytesPerRow:ce},[d,f]),k(u);let ue=d,de=f,fe=o===`bgra8unorm`;return le.mapAsync(GPUMapMode.READ).then(()=>{let e=new Uint8Array(le.getMappedRange()),t=new Uint8Array(ue*de*4);for(let n=0;n<de;n++)t.set(e.subarray(n*ce,n*ce+ue*4),n*ue*4);if(le.unmap(),le.destroy(),fe)for(let e=0;e<t.length;e+=4){let n=t[e];t[e]=t[e+2],t[e+2]=n}return t})},hold(e){if(D=e,!e&&!E&&O.length)for(let e of O.splice(0))e()},destroy(){if(!ie){ie=!0,D=!1,E&&!h&&k(null);for(let e of[...w])e.destroy();p?.destroy(),p=null,P?.destroy(),P=null,B?.destroy(),B=null,T.clear(),O.length=0;try{a.unconfigure()}catch{}i.destroy(),h=!0,Ki.close()}}}}async function aa(){let e=!1,t=!1,n=``,r=typeof navigator<`u`?navigator.gpu:void 0;if(!r)n=`no navigator.gpu`;else try{e=!!await r.requestAdapter(),e||(n=`WebGPU present but no adapter`)}catch(e){n=`WebGPU adapter failed: ${e.message}`}try{let e=(typeof OffscreenCanvas<`u`?new OffscreenCanvas(1,1):typeof document<`u`?document.createElement(`canvas`):null)?.getContext(`webgl2`);t=!!e,e?.getExtension(`WEBGL_lose_context`)?.loseContext()}catch{t=!1}let i=e?`webgpu`:t?`webgl2`:null,a=e?`WebGPU is available`:t?`WebGL2 (${n||`WebGPU unavailable`})`:`${n}; no WebGL2 either`;return{api:i,webgpu:e,webgl2:t,reason:a}}async function oa(e,t=`auto`){let n=``;if(t===`auto`||t===`webgpu`)try{return{graphics:await ia(e),reason:t===`webgpu`?`WebGPU, as asked`:`WebGPU (auto: available here)`}}catch(e){if(n=e.message,t===`webgpu`){let e=await aa();throw new Ln(`webgpu`,{webgpu:!1,webgl2:e.webgl2,reason:n})}}try{return{graphics:Zi(e),reason:t===`webgl2`?`WebGL2, as asked`:`WebGL2 (auto: ${n||`WebGPU unavailable`})`}}catch(e){throw new Ln(t,{webgpu:!1,webgl2:!1,reason:`${n?n+`; `:``}${e.message}`})}}const sa=()=>typeof matchMedia==`function`&&matchMedia(`(pointer: coarse)`).matches;function ca(e,t={}){let n=t.deviceMemory??(typeof navigator<`u`?navigator.deviceMemory:void 0)??8,r=t.cores??(typeof navigator<`u`?navigator.hardwareConcurrency:void 0)??8,i=n<=3||r<=2?`low`:n<=6||r<=4?`mid`:`high`,a=t.touch??sa(),o=t.quality?`as asked`:e.api===`webgl2`?`the WebGL2 default`:a?`a touch device`:i===`low`?`${n} GB of memory, ${r} cores`:``,s=t.quality??(e.api===`webgl2`||i===`low`||a?`lite`:`realistic`),c=s===`lite`;return{quality:s,tier:i,touch:a,why:o,maxPixelRatio:c?1.5:2,tileLayers:Math.min(e.maxLayers,c||i===`low`?128:192),tilePx:c?384:280,budget:a||i===`low`?{frameMs:22}:null}}var la=class{graphics;canvas;frameBlock;frameData=/* @__PURE__ */ new Float32Array(112);cssWidth=1;cssHeight=1;pixelRatio=1;clock;split=null;frames=0;cpuMs=0;tickMs=0;submitMs=0;drawn=0;skipped=0;rested=0;resting=!1;still=0;o;raf=0;last=0;running=!1;disposed=!1;ro=null;drawnAt=/* @__PURE__ */ new Float64Array(240);drawnN=0;restTimer=0;restFrom=0;idleFrom=-1;due=!1;loop=null;reads=[];constructor(e,t,n){this.canvas=e,this.graphics=t,this.o=n,this.clock=n.clock??`real`,this.frameBlock=t.buffer(`uniform`,448,`frame`),this.measure(),typeof ResizeObserver<`u`&&`getBoundingClientRect`in e&&(this.ro=new ResizeObserver(()=>{this.measure(),this.wake()}),this.ro.observe(e))}pin(e){this.pinned=e,e?(this.cssWidth=e[0]/this.pixelRatio,this.cssHeight=e[1]/this.pixelRatio,this.graphics.resize(e[0],e[1])):this.measure()}pinned=null;measure(){if(this.pinned)return;let e=this.canvas,t=typeof e.getBoundingClientRect==`function`?e.getBoundingClientRect():{width:e.width,height:e.height};this.cssWidth=Math.max(1,t.width||e.width),this.cssHeight=Math.max(1,t.height||e.height);let n=this.o.pixelRatio??(this.pixelRatio>0&&this.drawn>0?this.pixelRatio:this.fullRatio);this.pixelRatio=n,this.graphics.resize(Math.round(this.cssWidth*n),Math.round(this.cssHeight*n))}start(){if(this.running||this.disposed||this.clock===`manual`)return;this.running=!0,this.last=0,this.idleFrom=-1;let e=t=>{if(this.raf=0,!this.running)return;let n=this.last?t-this.last:16.7;if(this.last=t,n>50&&this.frames>2)try{this.o.onLong?.(n)}catch{}let r=!1;try{let e=typeof this.clock==`object`;this.resolution(t),!e&&!this.due&&!this.reads.length&&this.o.needsFrame&&!this.o.needsFrame()?(this.skipped++,this.idleDt+=Math.min(250,n),this.drewLast=!1):(this.due=!1,this.step(e?1e3/this.clock.fps:Math.min(250,n+this.idleDt)),this.idleDt=0,r=!0,this.drewLast&&!e&&this.o.onFrame&&this.o.onFrame(n),this.drewLast=!0)}catch(e){this.o.onError?.(e)}let i=this.o.rest,a=i===!1?1/0:i?.after??500;r?this.idleFrom=-1:this.idleFrom<0&&(this.idleFrom=t),!this.raf&&(this.o.needsFrame&&typeof this.clock!=`object`&&this.idleFrom>=0&&(t-this.idleFrom>=a||a!==1/0&&this.o.restNow?.())?this.rest():this.raf=requestAnimationFrame(e))};this.loop=e,this.raf=requestAnimationFrame(e)}rest(){if(!this.running||this.restTimer)return;this.resting||(this.resting=!0,this.restFrom=typeof performance<`u`?performance.now():Date.now());let e=this.o.rest,t=e===!1?250:e?.look??250,n=Math.max(0,this.o.nextIn?.()??1/0),r=this.o.busy?.()?32:1/0;this.restTimer=setTimeout(()=>{if(this.restTimer=0,!this.running||!this.resting)return;let e=!1;try{e=!!this.reads.length||!this.o.needsFrame||this.o.needsFrame()}catch(t){this.o.onError?.(t),e=!0}e?(this.due=!0,this.wake()):this.rest()},Math.min(t,n,r))}wake(){if(this.running&&!this.disposed){if(this.restTimer&&=(clearTimeout(this.restTimer),0),this.idleFrom=-1,this.resting){let e=typeof performance<`u`?performance.now():Date.now();this.rested+=e-this.restFrom,this.idleDt+=Math.min(250,e-this.restFrom),this.resting=!1,this.last=0,this.drewLast=!1}!this.raf&&this.loop&&(this.raf=requestAnimationFrame(this.loop))}}get fps(){let e=typeof performance<`u`?performance.now():Date.now(),t=this.drawnAt,n=Math.min(this.drawnN,t.length),r=0;for(let i=0;i<n;i++)e-t[i]<1e3&&r++;return r}idleDt=0;drewLast=!1;cap=null;get antialiased(){let e=this.o.antialias??`auto`;if(e!==`auto`)return e;let t=Math.min(this.o.maxPixelRatio,typeof devicePixelRatio==`number`?devicePixelRatio:1);return this.o.pixelRatio===void 0&&this.pixelRatio<t-.01}get fullRatio(){return Math.min(this.o.maxPixelRatio,typeof devicePixelRatio==`number`?devicePixelRatio:1,this.cap??1/0)}setCap(e){this.cap=e,this.o.pixelRatio===void 0&&(this.pixelRatio=this.fullRatio,this.graphics.resize(Math.round(this.cssWidth*this.pixelRatio),Math.round(this.cssHeight*this.pixelRatio)))}resolution(e){let t=this.o.movingPixelRatio??0;if(!t||this.o.pixelRatio!==void 0)return;let n=this.fullRatio;if(t>=n){this.pixelRatio!==n&&(this.pixelRatio=n,this.graphics.resize(Math.round(this.cssWidth*n),Math.round(this.cssHeight*n)));return}let r=this.o.moving?.()??!1;r&&(this.still=e);let i=r||e-this.still<300?t:n;i!==this.pixelRatio&&(this.pixelRatio=i,this.graphics.resize(Math.round(this.cssWidth*i),Math.round(this.cssHeight*i)))}stop(){this.running=!1,this.raf&&cancelAnimationFrame(this.raf),this.raf=0,this.restTimer&&clearTimeout(this.restTimer),this.restTimer=0,this.resting=!1}get isRunning(){return this.running}step(e){if(this.disposed)throw new K(`E-WORLD-003`,`the world was disposed`,{fix:`make a new world`});let n=typeof performance<`u`?performance.now():0,r=this.graphics;r.hold?.(!0);let i,a=n,o=[];try{let t=this.o.tick(e);a=typeof performance<`u`?performance.now():0,o=this.reads.splice(0),r.resize(Math.round(this.cssWidth*this.pixelRatio),Math.round(this.cssHeight*this.pixelRatio));let n=this.split,s=this.o.clear??[0,0,0,1];if(n){let e=r.width,t=r.height,a=Math.round(Math.max(0,Math.min(1,n.at))*e);r.frame(n.left,{clear:s,scissor:[0,0,a,t]}),i=r.frame(n.right,{clear:s,keep:!0,scissor:[a,0,e-a,t],read:o.length>0})}else i=r.frame(t,{clear:s,read:o.length>0,aa:this.antialiased})}finally{r.hold?.(!1),t()}for(let e of o)e(i);this.frames++,this.drawn++;let s=typeof performance<`u`?performance.now():0;return this.drawnAt[this.drawnN++%this.drawnAt.length]=s,this.cpuMs=s-n,this.tickMs=a-n,this.submitMs=s-a,i}read(){return new Promise((e,t)=>{if(this.reads.push(n=>n.then(t=>e({width:this.graphics.width,height:this.graphics.height,pixels:t}),t)),this.running)this.wake();else try{this.step(0)}catch(e){t(e)}})}dispose(){this.disposed||(this.stop(),this.disposed=!0,this.ro?.disconnect(),this.ro=null,this.graphics.destroy())}};const ua={sky:-1,basemap:0,raster:1,area:2,line:3,extrusion:4,shape:4,custom:4,point:5,label:6};function da(e){let t=new Map(e.map(e=>[e.id,e]));for(let n of e)for(let e of[n.above,n.below])if(e!==void 0&&!t.has(e))throw new K(`E-WORLD-002`,`layer '${n.id}' is ${n.above===e?`above`:`below`} '${e}', which is not in the world`,{where:n.id,fix:`add '${e}' first, or remove the ${n.above===e?`above`:`below`}`});let n=e=>e.order??ua[e.kind],r=[...e].filter(e=>e.above===void 0&&e.below===void 0).sort((e,t)=>n(e)-n(t)||e.added-t.added),i=e.filter(e=>e.above!==void 0||e.below!==void 0).sort((e,t)=>e.added-t.added);for(;i.length;){let e=[];for(let t of i){let n=t.above??t.below,i=r.findIndex(e=>e.id===n);if(i<0)e.push(t);else if(t.above!==void 0){let e=i+1;for(;e<r.length&&r[e].above===n;)e++;r.splice(e,0,t)}else r.splice(i,0,t)}if(e.length===i.length)throw new K(`E-WORLD-002`,`layers ${e.map(e=>`'${e.id}'`).join(`, `)} are placed relative to each other in a circle`,{fix:`give one of them an order number instead of above / below`});i=e}return r}const fa={$schema:`https://json-schema.org/draft/2020-12/schema`,$id:`https://aslnut.dev/schema/state-1.json`,title:`aslnut world state`,type:`object`,required:[`v`,`view`,`elements`,`selection`],additionalProperties:!1,properties:{v:{const:1},note:{type:`string`},view:{type:`object`,required:[`lon`,`lat`,`zoom`,`tilt`,`bearing`],additionalProperties:!1,properties:{lon:{type:`number`,minimum:-180,maximum:180},lat:{type:`number`,minimum:-90,maximum:90},zoom:{type:`number`,minimum:-2,maximum:26},tilt:{type:`number`,minimum:0,maximum:90},bearing:{type:`number`}}},time:{type:`string`,format:`date-time`},observation:{type:`object`,additionalProperties:!1,properties:{level:{type:`string`}}},render:{type:`object`,additionalProperties:!1,properties:{quality:{enum:[`lite`,`realistic`]}}},elements:{type:`object`,propertyNames:{pattern:`^[A-Za-z0-9_-]+$`},additionalProperties:{type:`object`,additionalProperties:!1,properties:{shown:{type:`boolean`},params:{type:`object`,additionalProperties:{type:[`number`,`string`,`boolean`,`array`],items:{type:`number`}}},filter:{type:[`object`,`null`]}}}},params:{type:`object`,additionalProperties:{type:[`number`,`string`,`boolean`,`array`],items:{type:`number`}}},selection:{type:`array`,items:{type:`string`}},toggles:{type:`object`,additionalProperties:{type:`boolean`}}}},Z=(e,t,n)=>new K(`E-STATE-001`,`${e}: ${t}`,{where:e,fix:n}),pa=e=>!!e&&typeof e==`object`&&!Array.isArray(e),ma=e=>typeof e==`number`&&Number.isFinite(e),ha=e=>ma(e)||typeof e==`string`||typeof e==`boolean`||Array.isArray(e)&&e.every(ma);function ga(e){if(!pa(e))throw Z(`state`,`is not an object`,`a state is { v: 1, view, elements, selection }`);for(let t of Object.keys(e))if(!(t in fa.properties))throw Z(`state.${t}`,`is not a state field`,`use ${Object.keys(fa.properties).join(`, `)}`);if(e.v!==1)throw Z(`state.v`,`is ${JSON.stringify(e.v)}`,`this aslnut reads version 1 (v: 1)`);if(e.note!==void 0&&typeof e.note!=`string`)throw Z(`state.note`,`is not text`,`write a string`);let t=e.view;if(!pa(t))throw Z(`state.view`,`is missing`,`give { lon, lat, zoom, tilt, bearing }`);for(let e of[`lon`,`lat`,`zoom`,`tilt`,`bearing`])if(!ma(t[e]))throw Z(`state.view.${e}`,`is ${JSON.stringify(t[e])}`,`give a number`);for(let e of Object.keys(t))if(![`lon`,`lat`,`zoom`,`tilt`,`bearing`].includes(e))throw Z(`state.view.${e}`,`is not a view field`,`use lon, lat, zoom, tilt, bearing`);if(t.lon<-180||t.lon>180)throw Z(`state.view.lon`,`${t.lon} is outside -180..180`,`longitude first, then latitude`);if(t.lat<-90||t.lat>90)throw Z(`state.view.lat`,`${t.lat} is outside -90..90`,`did you swap lon and lat?`);if(t.zoom<-2||t.zoom>26)throw Z(`state.view.zoom`,`${t.zoom} is outside -2..26`,`a web-map zoom level`);if(t.tilt<0||t.tilt>90)throw Z(`state.view.tilt`,`${t.tilt} is outside 0..90`,`degrees from straight down`);if(e.time!==void 0&&(typeof e.time!=`string`||Number.isNaN(Date.parse(e.time))))throw Z(`state.time`,`${JSON.stringify(e.time)} is not a date`,`write an ISO 8601 time, like '2026-10-12T04:00:00Z'`);if(e.observation!==void 0){if(!pa(e.observation))throw Z(`state.observation`,`is not an object`,`give { level }`);for(let t of Object.keys(e.observation))if(t!==`level`)throw Z(`state.observation.${t}`,`is not an observation field`,`use level (the quality is render.quality)`);if(e.observation.level!==void 0&&typeof e.observation.level!=`string`)throw Z(`state.observation.level`,`is ${JSON.stringify(e.observation.level)}`,`a level's name, like 'street'`)}if(e.render!==void 0){if(!pa(e.render))throw Z(`state.render`,`is not an object`,`give { quality }`);for(let t of Object.keys(e.render))if(t!==`quality`)throw Z(`state.render.${t}`,`is not a render field`,`use quality`);if(e.render.quality!==void 0&&e.render.quality!==`lite`&&e.render.quality!==`realistic`)throw Z(`state.render.quality`,`is ${JSON.stringify(e.render.quality)}`,`use 'lite' or 'realistic'`)}if(!pa(e.elements))throw Z(`state.elements`,`is missing`,`give { [element id]: { shown, params, filter } } ({} for none)`);for(let[t,n]of Object.entries(e.elements)){let e=`state.elements.${t}`;if(!/^[A-Za-z0-9_-]+$/.test(t))throw Z(e,`is not an element id`,`letters, digits, '-' and '_'`);if(!pa(n))throw Z(e,`is not an object`,`give { shown, params, filter }`);for(let t of Object.keys(n))if(![`shown`,`params`,`filter`].includes(t))throw Z(`${e}.${t}`,`is not an element field`,`use shown, params, filter`);if(n.shown!==void 0&&typeof n.shown!=`boolean`)throw Z(`${e}.shown`,`is ${JSON.stringify(n.shown)}`,`true or false`);if(n.params!==void 0){if(!pa(n.params))throw Z(`${e}.params`,`is not an object`,`give { name: value }`);for(let[t,r]of Object.entries(n.params))if(!ha(r))throw Z(`${e}.params.${t}`,`is ${JSON.stringify(r)}`,`a number, numbers, text or a boolean`)}if(n.filter!==void 0&&n.filter!==null&&!pa(n.filter))throw Z(`${e}.filter`,`is not an object`,`give { prop: value } (or null for all)`)}if(e.params!==void 0){if(!pa(e.params))throw Z(`state.params`,`is not an object`,`give { key: value }`);for(let[t,n]of Object.entries(e.params))if(!ha(n))throw Z(`state.params.${t}`,`is ${JSON.stringify(n)}`,`a number, numbers, text or a boolean`)}if(!Array.isArray(e.selection)||!e.selection.every(e=>typeof e==`string`))throw Z(`state.selection`,`is not a list of ids`,`give ["id", …] ([] for none)`);if(e.toggles!==void 0){if(!pa(e.toggles))throw Z(`state.toggles`,`is not an object`,`give { name: true }`);for(let[t,n]of Object.entries(e.toggles))if(typeof n!=`boolean`)throw Z(`state.toggles.${t}`,`is ${JSON.stringify(n)}`,`true or false`)}return e}const _a=e=>Math.round(e*1e6)/1e6,va=e=>Math.round(e*1e3)/1e3;function ya(e,t={}){let n=e.camera.view,r=t.elements?new Set(t.elements):null,i={},a={},o=e.params(),s=e.state;for(let t of e.list){if(t.kind===`sky`||r&&!r.has(t.id))continue;let e=t.forced;if(e===null)try{e=j(t.props.when,s,`layer '${t.id}'`)>.5}catch{e=t.target>.5}let n={shown:e},a={};for(let[e,n]of Object.entries(o))e.startsWith(`${t.id}.`)&&ha(n.value)&&(a[e.slice(t.id.length+1)]=n.value);Object.keys(a).length&&(n.params=a);let c=t.props.filter;c&&typeof c==`object`&&(n.filter=JSON.parse(JSON.stringify(c))),i[t.id]=n}let c=new Set(e.list.map(e=>e.id));for(let[e,t]of Object.entries(o))!c.has(e.split(`.`)[0])&&ha(t.value)&&(a[e]=t.value);return{v:1,...t.note?{note:t.note}:{},view:{lon:_a(n.lon),lat:_a(n.lat),zoom:va(n.zoom),tilt:va(n.tilt),bearing:va((n.bearing%360+360)%360)},...t.time===!1?{}:{time:e.time.toISOString()},...e.level===null?{}:{observation:{level:e.level}},render:{quality:e.state.render.quality},elements:i,...Object.keys(a).length?{params:a}:{},selection:[...e.user.selection],...Object.keys(e.user.toggles).length?{toggles:{...e.user.toggles}}:{}}}async function ba(e,t,n={}){let r=ga(t),i={missing:[],refused:[]};for(let[t,n]of Object.entries(r.elements)){let r;try{r=e.element(t)}catch{i.missing.push(t);continue}n.shown!==void 0&&(r.forced=n.shown);for(let[r,a]of Object.entries(n.params??{}))try{e.set(`${t}.${r}`,a)}catch{i.refused.push(`${t}.${r}`)}n.filter!==void 0&&r.setProps({filter:n.filter})}if(n.others===`hide`)for(let t of e.list)t.kind!==`sky`&&t.kind!==`basemap`&&!(t.id in r.elements)&&(t.forced=!1);for(let[t,n]of Object.entries(r.params??{}))try{e.set(t,n)}catch{i.refused.push(t)}if(e.select(r.selection),r.toggles)for(let[t,n]of Object.entries(r.toggles))e.toggle(t,n);r.time&&e.setTime(new Date(r.time)),r.render?.quality&&e.setQuality(r.render.quality),r.observation?.level!==void 0&&e.level!==null&&e.setLevel(r.observation.level);let a={lon:r.view.lon,lat:r.view.lat,zoom:r.view.zoom,tilt:r.view.tilt,bearing:r.view.bearing};return n.seconds===0?e.go(a):await e.fly(a,{seconds:n.seconds,ease:n.ease??`cinematic`,path:n.path}),i}function xa(e,t){let n=[],r=(e,t,i)=>{if(pa(e)&&pa(t))for(let n of/* @__PURE__ */ new Set([...Object.keys(e),...Object.keys(t)]))r(e[n],t[n],i?`${i}.${n}`:n);else JSON.stringify(e)!==JSON.stringify(t)&&n.push({path:i,from:e,to:t})};return r(e,t,``),n}const Sa=e=>typeof btoa==`function`?btoa(e):Buffer.from(e,`binary`).toString(`base64`),Ca=e=>typeof atob==`function`?atob(e):Buffer.from(e,`base64`).toString(`binary`);function wa(e){let t=new TextEncoder().encode(JSON.stringify(ga(e))),n=``;for(let e of t)n+=String.fromCharCode(e);return Sa(n).replace(/\+/g,`-`).replace(/\//g,`_`).replace(/=+$/,``)}function Ta(e){let t;try{let n=Ca(e.replace(/-/g,`+`).replace(/_/g,`/`)+`===`.slice((e.length+3)%4));t=JSON.parse(new TextDecoder().decode(Uint8Array.from(n,e=>e.charCodeAt(0))))}catch{throw Z(`state`,`could not be read from the text`,`pass what encode() or toURL() made`)}return ga(t)}function Ea(e,t=typeof location<`u`?location.href:`https://example.org/`,n=`state`){let r=new URL(t);return r.searchParams.set(n,wa(e)),r.toString()}function Da(e=typeof location<`u`?location.href:``,t=`state`){let n=new URL(e,`https://example.org/`).searchParams.get(t);return n?Ta(n):null}const Oa={capture:ya,apply:ba,diff:xa,validate:ga,encode:wa,decode:Ta,toURL:Ea,fromURL:Da,schema:fa},ka=Math.PI/180,Aa=Math.PI*(3-Math.sqrt(5)),ja={broad:.58,narrow:.35,needle:.14},Ma=(e,t,n=1)=>[e[0]+t[0]*n,e[1]+t[1]*n,e[2]+t[2]*n],Na=e=>{let t=Math.hypot(e[0],e[1],e[2])||1;return[e[0]/t,e[1]/t,e[2]/t]},Pa=(e,t)=>[e[1]*t[2]-e[2]*t[1],e[2]*t[0]-e[0]*t[2],e[0]*t[1]-e[1]*t[0]];var Fa=class{s;constructor(e){this.s=e*2654435761>>>0||1}next(){let e=this.s;return e^=e<<13,e^=e>>>17,e^=e<<5,this.s=e>>>0,this.s/4294967296}range(e,t){return e+(t-e)*this.next()}};function Ia(e,t,n){let r=Math.abs(e[1])<.95?[0,1,0]:[1,0,0],i=Na(Pa(e,r)),a=Pa(e,i),o=Math.sin(t),s=Math.cos(t);return Na([e[0]*s+(i[0]*Math.cos(n)+a[0]*Math.sin(n))*o,e[1]*s+(i[1]*Math.cos(n)+a[1]*Math.sin(n))*o,e[2]*s+(i[2]*Math.cos(n)+a[2]*Math.sin(n))*o])}function La(e){if(!(e?.trunk?.height>0)||!(e.trunk.radius>0)||!Array.isArray(e.levels))throw new K(`E-DATA-001`,`recipes.branching needs { trunk: { height, radius }, levels: [...] }`,{fix:`recipes.branching({ trunk: { height: 10, radius: 0.3 }, levels: [{ count: 5, angle: 45, length: 0.5, taper: 0.6 }] })`});let t=new Fa(e.seed??1),n=e.trunk.height,r=e.crown??`round`,i=e.trunk.clear??.35,a=[],o=(e,n,r,i,o,s,c)=>{let l=s===0?6:s===1?4:2,u=[e],d=n,f=e;for(let e=1;e<=l;e++)d=Na(Ma(d,[0,1,0],c*ka/l)),d=Na(Ma(d,[t.range(-.06,.06),0,t.range(-.06,.06)])),f=Ma(f,d,r/l),u.push(f);let p={pts:u,r0:i,r1:o,level:s,length:r,dir:n};return a.push(p),p},s=(e.trunk.lean??0)*ka,c=t.next()*Math.PI*2;if(e.method===`colonize`)Ra(e,t,a);else{let a=[o([0,0,0],Na([Math.sin(s)*Math.cos(c),Math.cos(s),Math.sin(s)*Math.sin(c)]),n,e.trunk.radius,e.trunk.radius*.25,0,0)];e.levels.forEach((e,n)=>{let s=[];for(let c of a){let a=e.start??(n===0?i:.25),l=t.next()*Math.PI*2;for(let i=0;i<e.count;i++){let u=a+(1-a)*((i+.5)/e.count)*(n===0?.92:1),d=za(c,u),f=n>0?1:r===`conical`?1.25-u:r===`columnar`?.45:r===`vase`?.6+.6*u:.55+Math.sin(Math.PI*Math.min(1,(u-a)/(1-a)*.9+.1))*.6,p=c.length*e.length*f*t.range(.8,1.15),m=Ia(c.dir,(e.angle+t.range(-8,8))*ka,l+i*Aa*2+t.range(-.3,.3)),h=c.r0+(c.r1-c.r0)*u;s.push(o(d,m,p,Math.max(.01,h*e.taper),Math.max(.006,h*e.taper*.3),n+1,e.bend??(n===0&&r===`vase`?25:10)))}}a=s})}let l=e.leaves??{},u=l.kind??`broad`,d=l.size??Math.max(.5,n*.08),f=l.density??3,p=(l.colors??[`#4f7a3a`,`#5b8a3f`,`#466f35`]).map(e=>e),m=e.levels.length,h=a.filter(e=>e.level>=Math.max(1,m-1)),g=[1/0,1/0,1/0],_=[-1/0,-1/0,-1/0];for(let e of h)for(let t of e.pts)g=[Math.min(g[0],t[0]),Math.min(g[1],t[1]),Math.min(g[2],t[2])],_=[Math.max(_[0],t[0]),Math.max(_[1],t[1]),Math.max(_[2],t[2])];let v=[(g[0]+_[0])/2,(g[1]+_[1])/2,(g[2]+_[2])/2],y=[];for(let n of e.leaves?h:[]){let e=Math.max(1,Math.round(n.length*f*(n.level===m?1:.6)));for(let r=0;r<e;r++){let i=za(n,.3+.7*((r+t.next())/e)),a=Na(Ma(Ma(i,v,-1),[t.range(-.4,.4),.35,t.range(-.4,.4)])),o=Na([t.range(-1,1),t.range(-.4,1),t.range(-1,1)]);y.push({c:Ma(i,a,d*.15),n:a,plane:o,s:d*t.range(.8,1.2),rot:t.next()*Math.PI*2,colour:p[Math.floor(t.next()*p.length)]})}}let b=e.bark??`#6b5440`,x=u===`broad`?1:u===`narrow`?3:2,S=(e,t,n)=>{for(let r of a)r.level<=t&&Ba(e,r,n(r.level),b)},C=(e,t,n=1)=>{for(let r of t)Va(e,r.c,r.plane,r.n,r.s*n,r.rot,x,r.colour)},w=Ce(e=>{S(e,99,e=>e===0?7:e===1?5:3),C(e,y)}),T=Ce(e=>{S(e,1,e=>e===0?5:3),C(e,y.filter((e,t)=>t%3==0),1.65)}),E=Ce(e=>{S(e,0,()=>3),C(e,y.filter((e,t)=>t%12==0),3.2)}),D=Math.max(.5,Math.max(_[0]-g[0],_[2]-g[2])/2+d*.5),O=Math.PI*D*D,k=0;for(let e of y)k+=e.s*e.s*ja[u]*Math.abs(e.plane[1]);let A=Math.exp(k/O*-.9);return{levels:[w,T,E],height:Math.max(n,_[1]+d*.5),crownRadius:D,crownBase:g[1],transmissivity:A,counts:{cards:y.length,vertices:[w,T,E].map(e=>e.length/6)},spec:e}}function Ra(e,t,n){let r=e.trunk.height,i=e.crown??`round`,a=(e.trunk.clear??.35)*r,o=Math.max(1,e.levels.length),s=r*(i===`columnar`?.17:i===`conical`?.3:i===`vase`?.42:.38),c=r*1.2,l=(a+c)/2,u=(c-a)/2,d=[];for(;d.length<380;){let e=[t.range(-1,1),t.range(-1,1),t.range(-1,1)];if(e[0]*e[0]+e[1]*e[1]+e[2]*e[2]>1)continue;let n=i===`conical`?(1-e[1])/2+.15:i===`vase`?(1+e[1])/2+.25:1;d.push([e[0]*s*n,l+e[1]*u,e[2]*s*n])}let f=r*.055,p=s*1.1,m=f*1.6,h=[{p:[0,0,0],parent:-1,kids:0,r:0}];for(let e=f;e<a;e+=f)h.push({p:[0,e,0],parent:h.length-1,kids:0,r:0});for(let e=0;e<120&&d.length;e++){let e=/* @__PURE__ */ new Map;for(let t of d){let n=-1,r=p;for(let e=0;e<h.length;e++){let i=h[e].p,a=Math.hypot(t[0]-i[0],t[1]-i[1],t[2]-i[2]);a<r&&(r=a,n=e)}if(n<0)continue;let i=h[n].p,a=Na([t[0]-i[0],t[1]-i[1],t[2]-i[2]]),o=e.get(n)??[0,0,0];e.set(n,[o[0]+a[0],o[1]+a[1],o[2]+a[2]])}if(e.size){for(let[n,r]of e){let e=Na(Ma(r,[t.range(-.1,.1),.05,t.range(-.1,.1)]));h.push({p:Ma(h[n].p,e,f),parent:n,kids:0,r:0})}for(let e=d.length-1;e>=0;e--){let t=d[e];h.some(e=>Math.hypot(t[0]-e.p[0],t[1]-e.p[1],t[2]-e.p[2])<m)&&d.splice(e,1)}}else{let e=h[h.length-1];h.push({p:Ma(e.p,[0,1,0],f),parent:h.length-1,kids:0,r:0})}}for(let e of h)e.parent>=0&&h[e.parent].kids++;let g=Math.max(.008,e.trunk.radius*.06),_=new Float64Array(h.length);for(let e=h.length-1;e>=0;e--){let t=h[e];t.r=t.kids===0?g:_[e]**(1/2.5),t.parent>=0&&(_[t.parent]=_[t.parent]+t.r**2.5)}let v=e.trunk.radius/Math.max(1e-6,h[0].r);for(let e of h)e.r*=v;for(let t=1;t<h.length;t++){let r=h[t],i=h[r.parent],a=r.r/e.trunk.radius,s=a>.5?0:Math.min(o,Math.max(1,Math.round((1-Math.min(1,a*2))*o)));n.push({pts:[i.p,r.p],r0:i.r,r1:r.r,level:s,length:f,dir:Na(Ma(r.p,i.p,-1))})}}function za(e,t){let n=Math.max(0,Math.min(1,t))*(e.pts.length-1),r=Math.min(e.pts.length-2,Math.floor(n)),i=n-r,a=e.pts[r],o=e.pts[r+1];return[a[0]+(o[0]-a[0])*i,a[1]+(o[1]-a[1])*i,a[2]+(o[2]-a[2])*i]}function Ba(e,t,n,r){let i=[],a=t.pts.length;for(let e=0;e<a;e++){let r=t.pts[e],o=Na(Ma(t.pts[Math.min(a-1,e+1)],t.pts[Math.max(0,e-1)],-1)),s=Math.abs(o[1])<.95?[0,1,0]:[1,0,0],c=Na(Pa(o,s)),l=Pa(o,c),u=t.r0+(t.r1-t.r0)*(e/(a-1)),d=[];for(let e=0;e<n;e++){let t=e/n*Math.PI*2;d.push([r[0]+(c[0]*Math.cos(t)+l[0]*Math.sin(t))*u,r[1]+(c[1]*Math.cos(t)+l[1]*Math.sin(t))*u,r[2]+(c[2]*Math.cos(t)+l[2]*Math.sin(t))*u])}i.push(d)}for(let o=0;o+1<a;o++)for(let a=0;a<n;a++){let s=i[o][a],c=i[o][(a+1)%n],l=i[o+1][(a+1)%n],u=i[o+1][a],d=t.pts[o],f=t.pts[o+1],p=(e,t)=>Na(Ma(e,t,-1));e.smoothTri(s,p(s,d),c,p(c,d),l,p(l,f),r).smoothTri(s,p(s,d),l,p(l,f),u,p(u,f),r)}}function Va(e,t,n,r,i,a,o,s){let c=Math.abs(n[1])<.95?[0,1,0]:[1,0,0],l=Na(Pa(n,c)),u=Pa(n,l),d=Na(Ma([l[0]*Math.cos(a),l[1]*Math.cos(a),l[2]*Math.cos(a)],u,Math.sin(a))),f=Pa(n,d),p=i/2,m=(e,n)=>[t[0]+(d[0]*e+f[0]*n)*p,t[1]+(d[1]*e+f[1]*n)*p,t[2]+(d[2]*e+f[2]*n)*p];e.card(m(-1,-1),m(1,-1),m(1,1),m(-1,1),r,o,s)}function Ha(e,t,n){let r=Object.fromEntries(Object.entries(e.spec.params??{}).map(([e,t])=>[e,e in n?{...t,def:n[e]}:t]));return new Gn({...e.spec,id:`${e.id}:${t}`,params:r})}function Ua(e,t,n={}){let r=n.grid??.5,i=e=>r>0?Math.round(e/r)*r:e;return new Gn({id:n.id,params:n.params,budget:n.budget,build:(n,r,a)=>{for(let o of typeof t==`function`?t(r):t){let t=e[o.piece];if(!t)throw new K(`E-DATA-001`,`the kit has no piece '${o.piece}' (it has ${Object.keys(e).join(`, `)})`,{fix:`kit: { ${o.piece}: shape({ … }) }`});n.frame({at:[i(o.at[0]),i(o.at[1]),i(o.at[2])],turn:o.heading??0},e=>t.spec.build(e,t.params(o.params),a))}}})}const Wa={branching:La,kitbash:Ua,preset:Ha};var Ga=class{info;shape;presets;kinematics;states;named=/* @__PURE__ */ new Map;constructor(e){let t=e?.info;if(!t||typeof t.id!=`string`||!t.id||typeof t.name!=`string`||typeof t.version!=`string`||typeof t.licence!=`string`)throw new K(`E-DATA-001`,`a shape package needs info { id, name, version, licence } (got ${JSON.stringify(t??null)})`,{fix:`shapePackage({ info: { id: 'bin', name: 'Wheelie bin', version: '1.0.0', licence: 'Apache-2.0' }, shape: bin })`});if(!e.shape||typeof e.shape.make!=`function`)throw new K(`E-DATA-001`,`shape package '${t.id}' needs a shape`,{where:t.id,fix:`shapePackage({ info, shape: shape({ params, build }) })`});this.info=t,this.shape=e.shape,this.presets=e.presets??{},this.kinematics=e.kinematics??{},this.states=e.states??{}}get id(){return this.info.id}preset(e){let t=this.presets[e];if(!t)throw new K(`E-DATA-001`,`shape package '${this.id}' has no preset '${e}'`,{where:this.id,fix:`one of: ${Object.keys(this.presets).join(`, `)||`none`}`});let n=this.named.get(e);return n||this.named.set(e,n=Ha(this.shape,e,t)),n}make(e={},t=1){return typeof e==`string`?this.preset(e).make({},t):this.shape.make(e,t)}check(e={}){let t=[],n=this.shape.check(e),r={};t.push(...n.problems),/^[A-Za-z0-9.+-]+( (OR|AND) [A-Za-z0-9.+-]+)*$/.test(this.info.licence)||t.push(`licence '${this.info.licence}' is not an SPDX id`);let i=new Set(this.shape.make(e).parts.map(e=>e.name));for(let[n]of Object.entries(this.presets))try{let t=this.preset(n).make(e);r[n]={triangles:t.triangles};for(let e of t.parts)i.add(e.name)}catch(e){t.push(`preset '${n}': ${e.message}`)}for(let[e,n]of Object.entries(this.kinematics))i.has(n.part)||t.push(`joint '${e}' turns part '${n.part}', which the shape has not (${[...i].join(`, `)||`no parts`})`),n.limits&&!(n.limits[0]<=n.limits[1])&&t.push(`joint '${e}': limits ${n.limits} are not [least, most]`),n.kind===`wheel`&&!(typeof n.radius==`string`?n.radius in(this.shape.spec.params??{}):Number(n.radius)>0)&&t.push(`wheel '${e}' needs a radius (metres, or a param's name)`);for(let[e,n]of Object.entries(this.states))for(let[r,i]of Object.entries(n)){let n=this.kinematics[r];n?n.limits&&(i<n.limits[0]||i>n.limits[1])&&t.push(`state '${e}': ${r} = ${i} is outside its limits ${n.limits}`):t.push(`state '${e}' poses joint '${r}', which the package has not`)}return{ok:!t.length,problems:t,shape:n,presets:r}}};const Ka=e=>new Ga(e),qa=e=>e instanceof Ga||!!e&&typeof e.make==`function`&&!!e.info?.id&&typeof e.check==`function`;var Ja=class{by=/* @__PURE__ */ new Map;use(...e){for(let t of e){if(!qa(t))throw new K(`E-DATA-001`,`w.shapes.use takes shape packages (shapePackage({ info, shape }))`,{fix:`w.shapes.use(cars.package)`});let e=this.by.get(t.id);e&&e.shape!==t.shape&&e.shape.update(t.shape.spec),this.by.set(t.id,e?Object.assign(Object.create(Object.getPrototypeOf(t)),t,{shape:e.shape}):t)}return this}has(e){return this.by.has(e)}get(e){let t=this.by.get(e);if(!t)throw new K(`E-DATA-001`,`no shape package '${e}' here`,{where:e,fix:`w.shapes.use(pkg) or w.shapes.load(catalogue) first; here: ${[...this.by.keys()].join(`, `)||`none`}`});return t}make(e,t={},n=1){return this.get(e).make(t,n)}list(){return[...this.by.values()].map(e=>e.info)}async load(e,t={}){let n=new URL(e,typeof location<`u`?location.href:`file:///`),r=t.fetch?await t.fetch(n.href):await(async()=>{let t=await fetch(n.href);if(!t.ok)throw new K(`E-DATA-002`,`catalogue ${e}: ${t.status}`,{where:e,fix:`check the url`});return t.json()})();if(!r||!Array.isArray(r.packages))throw new K(`E-DATA-001`,`catalogue ${e} has no packages list`,{where:e,fix:`{ "packages": [{ "id": "car", "module": "./car/shape.js" }] }`});let i=t.import??(e=>import(
/* @vite-ignore */
e)),a=[];for(let o of r.packages){if(t.only&&!t.only.includes(o.id))continue;let r=await i(new URL(o.module,n).href),s=r.default??r.package;if(!qa(s))throw new K(`E-DATA-001`,`catalogue ${e}: '${o.module}' exports no shape package`,{where:o.id,fix:`export default shapePackage({ info, shape })`});if(s.id!==o.id)throw new K(`E-DATA-001`,`catalogue ${e}: '${o.module}' is package '${s.id}', listed as '${o.id}'`,{where:o.id,fix:`the same id in the catalogue and shape.json`});this.by.set(s.id,s),a.push(s.info)}return a}},Ya=class{swap=null;hidden=/* @__PURE__ */ new Map;counts={changes:0,swaps:0,released:0,waitedMs:0};levels;constructor(e){this.levels=e}begin(e,t,n,r){this.counts.changes++;let i=new Map(this.swap?.held??[]),a=/* @__PURE__ */ new Set,o=/* @__PURE__ */ new Set;for(let e of t){if(e.forced!==null)continue;let t=0;try{t=j(e.props.when,n,`element '${e.id}'`)}catch{t=0}let r=i.get(e)??e.target;r>0==t>0?i.has(e)&&t>0==r>0&&i.delete(e):t>0?(a.add(e),i.set(e,0),this.hidden.delete(e)):i.set(e,r)}this.swap=a.size||i.size?{to:e.to,entering:a,ready:o,held:i,since:r}:null}hold(e){return this.swap?.held.get(e)}work(e,t,n,r){let i=this.swap;if(!i)return!1;let a=r();for(let n of i.entering){if(i.ready.has(n))continue;if(r()-a>t)break;let o=!0;try{o=n.prepare(e)}catch{o=!0}o&&i.ready.add(n)}if(i.ready.size<i.entering.size&&n-i.since<8e3)return!1;this.counts.swaps++,this.counts.waitedMs+=n-i.since;for(let[e,t]of i.held)t>0&&this.hidden.set(e,{at:n,level:this.levels.recent[0]??``});return this.swap=null,!0}keep(e){if(!this.hidden.size)return;let t=this.levels.keep,n=this.levels.recent.slice(0,t.levels);for(let[r,i]of this.hidden)if(r.target>0)this.hidden.delete(r);else if(!(r.shown>0)&&!(e-i.at<t.seconds*1e3&&n.includes(i.level))){this.hidden.delete(r);try{r.letGo&&(r.letGo(),this.counts.released++)}catch{}}}get nextKeep(){let e=null;for(let t of this.hidden.values()){let n=t.at+this.levels.keep.seconds*1e3;(e===null||n<e)&&(e=n)}return e}forget(e){this.hidden.delete(e),this.swap?.held.delete(e),this.swap?.entering.delete(e)}},Xa=class{budget;o;ladder;dropped=[];over=0;room=0;wait;lastUp=-1e9;n=0;constructor(e,t,n){this.budget=e,this.o=t,this.ladder=t.ladder??n,this.wait=t.stepUpAfter??120}get drops(){return this.dropped.map(e=>e.label)}frame(e){let t=this.budget;this.n++;let n=t.frameMs!==void 0&&e.gapMs>t.frameMs*1.5?`frame ${e.gapMs.toFixed(0)} ms`:t.drawCalls!==void 0&&e.draws>t.drawCalls?`${e.draws} draws`:t.triangles!==void 0&&e.triangles>t.triangles?`${Math.round(e.triangles)} triangles`:t.memoryMB!==void 0&&e.gpuMB>t.memoryMB?`${e.gpuMB.toFixed(0)} MB`:``;if(n){if(this.room=0,this.over++,this.over>=(this.o.stepDownAfter??10)){this.over=0,this.n-this.lastUp<300&&(this.wait=Math.min(this.wait*2,(this.o.stepUpAfter??120)*8));for(let e of this.ladder){let t=e.down();if(t)return this.dropped.push({rung:e,label:t}),{step:`down`,dropped:this.drops,why:n}}}return null}let r=t.frameMs,i=(r===void 0||e.gapMs<r*1.2&&e.cpuMs<r*.5&&(e.gpuMs===null||e.gpuMs<r*.6))&&(t.drawCalls===void 0||e.draws<t.drawCalls*.8)&&(t.triangles===void 0||e.triangles<t.triangles*.8)&&(t.memoryMB===void 0||e.gpuMB<t.memoryMB*.85);return this.over=Math.max(0,this.over-.25),i?++this.room<this.wait||!this.dropped.length?null:(this.room=0,this.lastUp=this.n,this.dropped.pop().rung.up(),{step:`up`,dropped:this.drops,why:`room`}):(this.room=0,null)}setLadder(e){this.reset(),this.ladder=this.o.ladder??e}reset(){for(;this.dropped.length;)this.dropped.pop().rung.up();this.over=0,this.room=0,this.wait=this.o.stepUpAfter??120}},Za=class{level;o;q=[];timer=0;sid=Math.random().toString(36).slice(2,8);off=null;constructor(e){if(this.o=e,this.level=e.level??`warn`,typeof addEventListener==`function`){let e=()=>this.flush();addEventListener(`pagehide`,e),this.off=()=>removeEventListener(`pagehide`,e)}}send(e,t={},n=`info`){if(n===`info`&&this.level===`warn`)return;let r=typeof this.o.fields==`function`?this.o.fields():this.o.fields;this.q.push(JSON.stringify({sid:this.sid,t:Math.round(typeof performance<`u`?performance.now():Date.now()),...r,ev:e,...t})),this.q.length>400&&this.q.splice(0,this.q.length-400),this.timer||=setTimeout(()=>this.flush(),this.o.every??2e3)}flush(){if(this.timer&&clearTimeout(this.timer),this.timer=0,!this.q.length)return;let e=this.q.splice(0).join(`
`);try{this.o.send?this.o.send(this.o.url,e):typeof fetch==`function`&&fetch(this.o.url,{method:`POST`,body:e,keepalive:e.length<6e4}).catch(()=>{})}catch{}}dispose(){this.flush(),this.off?.(),this.off=null}};const Qa=e=>typeof e.setRule==`function`,$a={added:`#22c55e`,removed:`#ef4444`,changed:`#f59e0b`};var eo=class{w;all=/* @__PURE__ */ new Map([[`base`,{name:`base`,base:``,set:{},params:{},rules:{},label:`as it is`}]]);now=`base`;originals=/* @__PURE__ */ new Map;paramsBefore=/* @__PURE__ */ new Map;cmp=null;constructor(e){this.w=e}define(e,t){if(t===void 0){let t=this.all.get(e);if(!t)throw new K(`E-DATA-001`,`no scenario '${e}' (there are ${[...this.all.keys()].join(`, `)})`,{fix:`w.scenario('${e}', { set: { … } })`});return t}if(e===`base`)throw new K(`E-DATA-001`,`'base' is the world as it is: it cannot be changed`,{fix:`give your scenario another name: w.scenario('plan-a', { … })`});let n=t.base??`base`;if(!this.all.has(n))throw new K(`E-DATA-001`,`scenario '${e}' is based on '${n}', which is not defined`,{fix:`define '${n}' first`});let r={name:e,base:n,set:{...t.set??{}},params:{...t.params??{}},rules:{...t.rules??{}},label:t.label??e};return this.all.set(e,r),this.now===e&&this.use(e),r}list(){return[...this.all.keys()]}get current(){return this.now}resolved(e){let t=[],n=this.define(e);for(;n&&n.name!==`base`;){if(t.includes(n))throw new K(`E-DATA-001`,`scenarios are based on each other in a circle (${t.map(e=>e.name).join(` → `)})`,{fix:`base one of them on base`});t.unshift(n),n=this.all.get(n.base)}let r={set:{},params:{},rules:{}};for(let e of t){Object.assign(r.set,e.set),Object.assign(r.params,e.params);for(let[t,n]of Object.entries(e.rules??{}))r.rules[t]=typeof n==`object`&&n&&`rule`in n?n:{rule:n}}return r}use(e){let t=this.resolved(e);this.restore();for(let[n,r]of Object.entries(t.set)){let t=this.layersFor(n);t.length||this.w.error(new K(`E-DATA-001`,`scenario '${e}': no layer uses the source or is the layer '${n}'`,{fix:`set: { <source id or layer id>: data.geojson(…) }`}));for(let e of t)this.remember(e,`data`),e.setProps({data:r})}for(let[e,n]of Object.entries(t.params))this.paramsBefore.has(e)||this.paramsBefore.set(e,this.w.get(e)),this.w.set(e,n);for(let[n,r]of Object.entries(t.rules)){let t=this.w.element(n);!t||!Qa(t)?this.w.error(new K(`E-DATA-001`,`scenario '${e}': no field '${n}'`,{fix:`rules: { <field id>: rules.diffusion(…) }`})):(this.remember(t,`rule`),t.setRule(r.rule,r.params??{}).catch(e=>this.w.error(e)))}return this.now=e,this.w.invalidate(),this}async diff(e,t){let n=this.resolved(e),r=this.resolved(t),i={added:[],removed:[],changed:[],params:[],rules:[]},a={loader:this.w.loader,error:e=>this.w.error(e)},o=async(e,t)=>{let n=e??this.originalData(t);return Array.isArray(n)?n:n instanceof Te?(await n.start(a),await n.ready,n.features()):[]};for(let e of/* @__PURE__ */ new Set([...Object.keys(n.set),...Object.keys(r.set)])){let t=await o(n.set[e],e),a=await o(r.set[e],e),s=new Map(t.map(e=>[String(e.id),e])),c=new Map(a.map(e=>[String(e.id),e]));for(let[t,n]of c){let r=s.get(t);r?JSON.stringify([r.props,r.geometry])!==JSON.stringify([n.props,n.geometry])&&i.changed.push({source:e,id:n.id}):i.added.push({source:e,id:n.id})}for(let[t,n]of s)c.has(t)||i.removed.push({source:e,id:n.id})}for(let e of/* @__PURE__ */ new Set([...Object.keys(n.params),...Object.keys(r.params)])){let t=e in n.params?n.params[e]:this.paramsBefore.has(e)?this.paramsBefore.get(e):this.w.get(e),a=e in r.params?r.params[e]:this.paramsBefore.has(e)?this.paramsBefore.get(e):this.w.get(e);JSON.stringify(t)!==JSON.stringify(a)&&i.params.push({key:e,a:t,b:a})}for(let e of/* @__PURE__ */ new Set([...Object.keys(n.rules),...Object.keys(r.rules)])){let t=t=>t?typeof t.rule==`function`?`custom`:t.rule.name??`kernel`:this.originalRuleName(e);n.rules[e]?.rule!==r.rules[e]?.rule&&i.rules.push({field:e,a:t(n.rules[e]),b:t(r.rules[e])})}return i}compare(e,t,n={}){if(this.stopCompare(),e===null)return this;if(!t)throw new K(`E-DATA-001`,`compare needs two scenarios`,{fix:`w.scenario.compare('base', 'plan-a')`});let r=n.view??`split`,i=this.resolved(t);this.use(e);let a=[],o=[],s=[],c=(e,n)=>{let r=e.constructor,i=new r({...e.props,...n,id:`${e.id}--${t}`,above:e.id,order:void 0});return a.push(i),s.push([e,i]),i};for(let[e,n]of Object.entries(i.set))for(let i of this.layersFor(e)){if(r!==`difference`){c(i,{data:n});continue}o.push(i);let e=e=>t=>this.diffSets.get(`${i.id}:${e}`)?.has(String(t.id))?$a[e]:`rgba(0,0,0,0)`;c(i,{data:n,color:t=>this.diffSets.get(`${i.id}:added`)?.has(String(t.id))?$a.added:e(`changed`)(t)});let s=new i.constructor({...i.props,id:`${i.id}--${t}--removed`,above:i.id,order:void 0,color:e(`removed`)});a.push(s)}for(let[e,n]of Object.entries(i.rules)){let i=this.w.element(e);if(!i||!Qa(i))continue;let s=i,l=s.props.draw||{},u={...l,on:`surface`,flat:l.on!==`surface`||l.flat};this.remember(s,`draw`),s.setProps({draw:u});let d=c(s,{rule:n.rule,params:n.params??{},draw:u,init:s.state.slice(0)});if(d.time=s.time,s.rate>0?d.play(s.rate):d.pause(),r===`difference`){o.push(s,d);let e={...u,ramp:[`#2166ac`,`#67a9cf`,`#d1e5f0`,`#f7f7f7`,`#fddbc7`,`#ef8a62`,`#b2182b`],domain:l.diffDomain??[-1,1],min:-1e30},n=new s.constructor({...s.props,id:`${s.id}--${t}--difference`,rule:s.rule,init:0,rate:0,draw:e,gpu:!1,above:s.id,order:void 0});a.push(n);let r=()=>{let e=s.values(),t=d.values(),r=new Float32Array(e.length);for(let n=0;n<r.length;n++)r[n]=t[n]-e[n];n.reset(r)},i=setInterval(r,300);this.pendingOffs.push(()=>clearInterval(i)),r()}}for(let e of a)this.w.add(e);for(let e of o)this.remember(e,`when`),e.setProps({when:!1});r===`difference`&&this.markDiff(e,t).catch(e=>{this.cmp?.b===t&&this.w.error(e)});let l={at:n.at??.5,leftHides:/* @__PURE__ */ new Set,rightHides:/* @__PURE__ */ new Set};if(r!==`difference`)for(let[e,t]of s)l.leftHides.add(t),l.rightHides.add(e);return this.w.compareView=r===`difference`?null:l,this.cmp={a:e,b:t,view:r,twins:a,hidden:o,divider:r===`difference`?null:this.divider(l,r===`swipe`),offs:this.pendingOffs.splice(0)},this.w.invalidate(),this}setSplit(e){let t=this.w.compareView;return t&&(t.at=Math.max(0,Math.min(1,e)),this.placeDivider(),this.w.invalidate()),this}get comparing(){return this.cmp?{a:this.cmp.a,b:this.cmp.b,view:this.cmp.view}:null}stopCompare(){let e=this.cmp;if(e){this.cmp=null,this.w.compareView=null;for(let t of e.offs)t();for(let t of e.twins)this.w.remove(t.id);e.divider?.remove();for(let[e,t]of this.originals)`when`in t&&(e.setProps({when:t.when}),delete t.when),`draw`in t&&(e.setProps({draw:t.draw}),delete t.draw);this.w.invalidate()}}pendingOffs=[];diffSets=/* @__PURE__ */ new Map;async markDiff(e,t){let n=await this.diff(e,t);if(this.cmp?.a===e&&this.cmp?.b===t){for(let e of[`added`,`removed`,`changed`])for(let t of n[e])for(let n of this.layersFor(t.source)){let r=`${n.id}:${e}`;this.diffSets.has(r)||this.diffSets.set(r,/* @__PURE__ */ new Set),this.diffSets.get(r).add(String(t.id))}for(let e of this.cmp?.twins??[])e.setProps({});this.w.invalidate()}}divider(e,t){let n=this.w.canvasElement;if(!n||typeof document>`u`||!n.parentElement)return null;let r=document.createElement(`div`);Object.assign(r.style,{position:`absolute`,top:`0`,bottom:`0`,width:t?`22px`:`2px`,marginLeft:t?`-11px`:`-1px`,zIndex:`5`,cursor:t?`ew-resize`:`default`,touchAction:`none`,pointerEvents:t?`auto`:`none`,display:`flex`,justifyContent:`center`});let i=document.createElement(`div`);if(Object.assign(i.style,{width:`2px`,height:`100%`,background:`rgba(255,255,255,.9)`,boxShadow:`0 0 4px rgba(0,0,0,.6)`}),r.appendChild(i),getComputedStyle(n.parentElement).position===`static`&&(n.parentElement.style.position=`relative`),n.parentElement.appendChild(r),t){let e=!1;r.addEventListener(`pointerdown`,t=>{e=!0,r.setPointerCapture(t.pointerId),t.stopPropagation()}),r.addEventListener(`pointermove`,t=>{if(!e)return;let r=n.getBoundingClientRect();this.setSplit((t.clientX-r.left)/r.width),t.stopPropagation()}),r.addEventListener(`pointerup`,()=>{e=!1})}return this.dividerEl=r,this.placeDivider(),r}dividerEl=null;placeDivider(){let e=this.dividerEl,t=this.w.canvasElement,n=this.w.compareView;e&&t&&n&&(e.style.left=`${t.offsetLeft+n.at*t.clientWidth}px`,e.style.top=`${t.offsetTop}px`,e.style.height=`${t.clientHeight}px`,e.style.bottom=``)}isTwin(e){return!!this.cmp?.twins.includes(e)||this.building.has(e)}building=/* @__PURE__ */ new Set;layersFor(e){return this.w.elements.filter(t=>!this.isTwin(t)&&(t.id===e||t.props.data instanceof Te&&(this.originals.get(t)?.data??t.props.data).id===e))}originalData(e){let t=this.layersFor(e)[0];if(t)return this.originals.get(t)?.data??t.props.data}originalRuleName(e){let t=this.w.element(e);return!t||!Qa(t)?`—`:(this.originals.get(t)?.rule??t.rule).name}remember(e,t){let n=this.originals.get(e)??{};if(this.originals.set(e,n),!(t in n)){if(t===`data`)n.data=e.props.data;else if(t===`rule`){let t=e;n.rule=t.rule,n.params=Object.fromEntries(Object.keys(t.rule.params).map(e=>[e,t.rule.params[e].value]))}else t===`draw`?n.draw=e.props.draw:n.when=e.props.when}}restore(){for(let[e,t]of this.originals)`data`in t&&e.setProps({data:t.data}),t.rule&&Qa(e)&&e.setRule(t.rule,t.params??{}).catch(e=>this.w.error(e));for(let[e,t]of[...this.originals])delete t.data,delete t.rule,delete t.params,Object.keys(t).length||this.originals.delete(e);for(let[e,t]of this.paramsBefore)this.w.set(e,t);this.paramsBefore.clear()}};function to(e){let t=new eo(e),n=((e,n)=>t.define(e,n));return Object.defineProperties(n,{use:{value:e=>t.use(e)},diff:{value:(e,n)=>t.diff(e,n)},compare:{value:(e,n,r)=>t.compare(e,n,r)},setSplit:{value:e=>t.setSplit(e)},list:{value:()=>t.list()},current:{get:()=>t.current},comparing:{get:()=>t.comparing},manager:{value:t}}),n}var no=class{tris;x0;z0;x1;z1;cell;nx;nz;start;items;constructor(e,t=4){this.tris=e;let n=1/0,r=1/0,i=-1/0,a=-1/0;for(let t=0;t<e.length;t+=3){let o=e[t],s=e[t+1];o<n&&(n=o),o>i&&(i=o),s<r&&(r=s),s>a&&(a=s)}i>=n||(n=r=0,i=a=-1),this.x0=n,this.z0=r,this.x1=i,this.z1=a,this.cell=t;let o=this.nx=Math.max(1,Math.ceil((i-n)/t)+1),s=this.nz=Math.max(1,Math.ceil((a-r)/t)+1),c=e.length/9,l=new Int32Array(o*s+1),u=(i,a)=>{let s=i*9,c=Math.min(e[s],e[s+3],e[s+6]),l=Math.max(e[s],e[s+3],e[s+6]),u=Math.min(e[s+1],e[s+4],e[s+7]),d=Math.max(e[s+1],e[s+4],e[s+7]);for(let e=Math.floor((u-r)/t);e<=Math.floor((d-r)/t);e++)for(let r=Math.floor((c-n)/t);r<=Math.floor((l-n)/t);r++)a(e*o+r)};for(let e=0;e<c;e++)u(e,e=>l[e+1]++);for(let e=0;e<o*s;e++)l[e+1]+=l[e];let d=l.slice(0,o*s),f=new Int32Array(l[o*s]);for(let e=0;e<c;e++)u(e,t=>{f[d[t]++]=e});this.start=l,this.items=f}levelsAt(e,t,n){if(e<this.x0||e>this.x1||t<this.z0||t>this.z1)return;let r=Math.floor((t-this.z0)/this.cell)*this.nx+Math.floor((e-this.x0)/this.cell),i=this.tris;for(let a=this.start[r],o=this.start[r+1];a<o;a++){let r=this.items[a]*9,o=i[r],s=i[r+1],c=i[r+3]-o,l=i[r+4]-s,u=i[r+6]-o,d=i[r+7]-s,f=c*d-l*u;if(!f)continue;let p=((e-o)*d-(t-s)*u)/f,m=((t-s)*c-(e-o)*l)/f;if(p<-1e-6||m<-1e-6||p+m>1+1e-6)continue;let h=i[r+2]+p*(i[r+5]-i[r+2])+m*(i[r+8]-i[r+2]),g=!1;for(let e=0;e<n.length;e++)if(Math.abs(n[e]-h)<.001){g=!0;break}g||n.push(h)}}get bytes(){return this.tris.byteLength+this.start.byteLength+this.items.byteLength}};const ro=(e,t)=>new no(e,t);function io(e,t,n=NaN,r={}){if(!e.length)return t;let i=1/0;for(let t of e)t<i&&(i=t);if(!Number.isFinite(n))return i;let a=r.hidden??.5,o=r.headroom??2.2,s=Number.isNaN(t);for(let n of e)n>=t-a&&n<=t+o&&(s=!0);let c=s?NaN:t,l=s?1/0:Math.abs(t-n);for(let t of e){let e=Math.abs(t-n);e<l&&(l=e,c=t)}return c}const ao={dark:{grey:.95,tint:`#394c6a`,tintAmount:.8,brightness:.62,contrast:.9,ground:.55,sky:{amount:1,color:`#151c28`}},grey:{grey:1,brightness:1,contrast:.8,sky:{amount:.8,color:`#8e959c`}},light:{grey:1,tint:`#eef1f4`,tintAmount:.35,brightness:1.3,contrast:.55,sky:{amount:.85,color:`#e6eaee`}}},oo=e=>e<0?0:e>1?1:e,so=(e,t,n,r=0,i=1/0)=>{if(e===void 0)return t;if(typeof e!=`number`||!Number.isFinite(e)||e<r||e>i)throw new K(`E-DATA-001`,`theme: ${n} must be a number from ${r} to ${i===1/0?`any`:i}, got ${String(e)}`,{fix:`e.g. theme.grey({ ${n}: ${t} })`});return e};function co(e,t){let n={...t};if(so(n.grey,1,`grey`,0,1),so(n.tintAmount,.75,`tintAmount`,0,1),so(n.brightness,1,`brightness`,0,4),so(n.contrast,1,`contrast`,0,4),so(n.ground,1,`ground`,0,4),so(n.seconds,1,`seconds`,0),n.tint!==void 0&&L(n.tint),n.sky&&(so(n.sky.amount,1,`sky.amount`,0,1),n.sky.color!==void 0&&L(n.sky.color)),n.subjects!==void 0&&typeof n.subjects!=`function`&&!Array.isArray(n.subjects))throw new K(`E-DATA-001`,`theme: subjects are layer ids or your function of a layer`,{fix:`theme.dark({ subjects: ['transit'] }) · theme.dark({ subjects: (l) => l.id.startsWith('access') })`});let r=ve(n.ramp??`viridis`),i=Object.fromEntries(Object.entries(Rt).map(([e,t])=>[e,(e,n={})=>t(e,{...n,range:r})])),a=Array.isArray(n.subjects)?new Set(n.subjects):null,o=typeof n.subjects==`function`?n.subjects:null,s=lo(n);return{name:e,spec:n,ramp:r,scale:i,subject:e=>e.props.subject===!0||(a?a.has(e.id):o?!!o(e):!1),mute:(e,t=1)=>uo(e,s,t)}}function lo(e){let t=e.tint===void 0?[.5,.5,.5]:L(e.tint),n=e.sky?L(e.sky.color??`#151c28`):[0,0,0];return Float32Array.from([t[0],t[1],t[2],e.tint===void 0?0:e.tintAmount??.75,e.grey??1,e.brightness??1,e.contrast??1,e.ground??1,n[0],n[1],n[2],e.sky?e.sky.amount??1:0])}function uo(e,t,n){let r=[e[0],e[1],e[2]];if(!(n>0))return r;let i=.299*e[0]+.587*e[1]+.114*e[2];for(let a=0;a<3;a++){let o=e[a]+(i-e[a])*t[4];o+=(t[a]*2*i-o)*t[3],o=(o-.5)*t[6]+.5,o=Math.max(0,o*t[5]),r[a]=e[a]+(o-e[a])*n}return r}const fo=e=>(t={})=>co(e,{...ao[e],...t}),po={dark:fo(`dark`),grey:fo(`grey`),light:fo(`light`),none:()=>null,custom:(e={})=>co(`custom`,{sky:!1,...e}),define:(e,t=`custom`)=>(co(t,e),(n={})=>co(t,{sky:!1,...e,...n}))},mo=e=>!!e&&typeof e==`object`&&typeof e.subject==`function`&&typeof e.mute==`function`;var ho=class{shownTheme=null;want=null;k=0;v=/* @__PURE__ */ new Float32Array(12);k0=0;v0=/* @__PURE__ */ new Float32Array(12);kTo=0;to=/* @__PURE__ */ new Float32Array(12);p=1;seconds=1;clockMs=0;gen=0;started=!1;set(e,t){if(e!==null&&!mo(e))throw new K(`E-DATA-001`,`setTheme takes a theme (theme.dark(), theme.grey() …) or null`,{fix:`w.setTheme(theme.grey({ subjects: ['my-layer'] }))`});this.want=e,this.gen++,this.seconds=t??e?.spec.seconds??this.shownTheme?.spec.seconds??1,e&&(this.to.set(lo(e.spec)),(!this.shownTheme||this.k<=0)&&(this.v.set(this.to),this.shownTheme=e)),this.started=!1}get asked(){return this.want}get amount(){return this.k}get easing(){return this.started?this.p<1:!!(this.want||this.shownTheme)}get active(){return this.k>0}step(e,t){if(!this.shownTheme&&!this.want){this.p=1,this.started=!0;return}this.clockMs+=e;let n=this.want?j(this.want.spec.when,t,`theme`):0;if((!this.started||n!==this.kTo)&&(this.started=!0,this.k0=this.k,this.v0.set(this.v),this.kTo=n,this.p=0),this.p<1){this.p=this.seconds<=0?1:Math.min(1,this.p+Math.max(0,e)/1e3/this.seconds);let t=this.p*this.p*(3-2*this.p);if(this.k=this.k0+(this.kTo-this.k0)*t,this.want)for(let e=0;e<12;e++)this.v[e]=this.v0[e]+(this.to[e]-this.v0[e])*t}this.want?this.shownTheme=this.want:this.k<=0&&(this.shownTheme=null,this.gen++)}write(e,t,n=this.k){let r=this.v;e[t]=r[0],e[t+1]=r[1],e[t+2]=r[2],e[t+3]=r[3],e[t+4]=r[4],e[t+5]=r[5],e[t+6]=r[6],e[t+7]=this.shownTheme?n:0,e[t+8]=this.shownTheme?r[7]:0,e[t+9]=0,e[t+10]=0,e[t+11]=0}get sky(){let e=this.v;return!this.shownTheme||!(e[11]>0)||!(this.k>0)?null:{amount:oo(e[11]*this.k),color:[e[8],e[9],e[10]]}}subject(e){return!!this.shownTheme&&this.shownTheme.subject(e)}key(){return!this.shownTheme&&this.k<=0?``:this.p<1?`/th${this.gen}e${Math.floor(this.clockMs/125)}`:`/th${this.gen}k${this.k.toFixed(3)}`}};const go=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);let _o=0;var vo=class{id=`world-${++_o}`;camera;body;loader;renderer;graphics;profile;api;reason;views=/* @__PURE__ */ new Map;shapes=new Ja;user;time;timeIsNow;clockRate=1;zones;lastTimeLook=0;timeRest=0;get timeMs(){return this.time.getTime()+this.timeRest}qualityNow=null;qualityFn=null;elements=[];sorted=[];seq=0;levelsObj=null;gates=null;paramDefs=/* @__PURE__ */ new Map;readDefs=/* @__PURE__ */ new Map;pointDefs=/* @__PURE__ */ new Map;ev={};context;lastDraws=[];pickTarget=null;picking=!1;pointer=null;hoverWanted=!1;compareView=null;scenario=(e=>to({get elements(){return e.elements},get loader(){return e.loader},add:(...t)=>e.add(...t),remove:t=>e.remove(t),element:t=>e.elements.find(e=>e.id===t),get:t=>e.get(t),set:(t,n)=>e.set(t,n),invalidate:()=>e.invalidate(),error:t=>e.error(t),get compareView(){return e.compareView},set compareView(t){e.compareView=t},get canvasElement(){return e.canvasEl}}))(this);hoverNow=null;tip=null;canvasEl=null;tipOpts;nowMs=0;frameN=0;groundTarget=0;disposed=!1;ownLoader;detachControls=null;stateCache=null;holdMs=0;following=null;shot=null;loading=/* @__PURE__ */ new Set;errorHandler;groundMarks;groundMarksChosen;lightNow=null;shadowOpts=null;shadowTarget=null;shadowBuf=null;noShadow=null;shadowData=/* @__PURE__ */ new Float32Array(112);shadowStats={on:!1,size:0,casters:0,ms:0,radius:0,kept:!1};creditsOpts=null;creditsEl=null;creditsText=``;creditsAt=-1e9;credits(){let e=this.creditsOpts,t=this.canvasEl;if(!e||!t||typeof document>`u`)return;let n=this.nowReal();if(n-this.creditsAt<500&&!this.recordingNow)return;this.creditsAt=n;let r=this.attribution(),i=e.format?e.format(r):r.join(` · `);if(i!==this.creditsText){if(this.creditsText=i,!this.creditsEl){let n=document.createElement(`div`),r=e.position??`bottom-right`,i=t.parentElement;if(!i)return;getComputedStyle(i).position===`static`&&(i.style.position=`relative`),n.className=e.className??`aslnut-credits`,e.className||(Object.assign(n.style,{font:`10px/1.3 system-ui, sans-serif`,color:`rgba(255,255,255,.8)`,textShadow:`0 0 3px #000`,maxWidth:`72%`,pointerEvents:`auto`,cursor:`pointer`,whiteSpace:`nowrap`,overflow:`hidden`,textOverflow:`ellipsis`,textAlign:r.endsWith(`right`)?`right`:`left`}),n.addEventListener(`click`,()=>{let e=n.style.whiteSpace===`nowrap`;n.style.whiteSpace=e?`normal`:`nowrap`,n.style.background=e?`rgba(0,0,0,.55)`:``,n.style.padding=e?`4px 6px`:``,n.style.borderRadius=e?`6px`:``})),Object.assign(n.style,{position:`absolute`,zIndex:`5`,[r.startsWith(`bottom`)?`bottom`:`top`]:r.startsWith(`bottom`)?`calc(3px + env(safe-area-inset-bottom, 0px))`:`3px`,[r.endsWith(`right`)?`right`:`left`]:`6px`}),i.insertBefore(n,t.nextSibling),this.creditsEl=n}this.creditsEl.textContent=i,this.creditsEl.title=i}}formLayer=null;pickedById=/* @__PURE__ */ new Map;remember(e){let t=String(e.id);this.pickedById.delete(t),this.pickedById.set(t,e),this.pickedById.size>400&&this.pickedById.delete(this.pickedById.keys().next().value)}async formFor(e,t,n){if(!this.formLayer){if(!t||!n||!n.shape)return;let{FormMarks:e}=await import(`./forms-7rJCXouj.mjs`);if(this.disposed)return;this.formLayer||(this.formLayer=new e,this.add(this.formLayer))}let r=e=>{try{return this.element(e.element)}catch{return null}};this.formLayer.set(e,t,n,e=>r(e)?.placeOf(e)??(e.at?{at:[e.at[0],e.at[1],(e.at[2]??0)*this.exaggeration],turn:0,scale:[1,1,1]}:null)),this.invalidate()}posesOf(e){return e.map(e=>this.camera.framed(hr(this.resolve(e))))}planners(){return this.sorted.flatMap(e=>{let t=e.plan;return typeof t==`function`?[{l:e,plan:t.bind(e)}]:[]})}prefetchN=0;prefetch(e,t={}){let n=e.map(e=>typeof e==`object`&&`pitch`in e?(mr(e,`prefetch`),e):this.posesOf([e])[0]),r=++this.prefetchN,i=this.nowReal();this.aheadLevels=this.levelsOf(n),this.soonAt=-1e9;let a=this.sorted.filter(e=>typeof e.pumpAhead==`function`);for(let e of a)e.prefetch(n,{depth:t.depth});return new Promise(e=>{let n=-1,o=i,s=()=>{if(this.disposed||r!==this.prefetchN){e({needed:0,have:0});return}let c=0,l=0;for(let e of a){let t=e.pumpAhead();t&&(c+=t.needed,l+=t.have)}if(c>0&&l>=c||this.nowReal()-i>(t.timeout??3e4)||!a.length){for(let e of a)e.prefetch([]);this.recordingNow||(this.aheadLevels=null),e({needed:c,have:l})}else l===n?this.nowReal()-o>1e3&&(o=this.nowReal(),this.invalidate()):(n=l,o=this.nowReal()),setTimeout(s,100)};s()})}levelsOf(e){let t=this.levelsObj;if(!t)return null;let n=t.index,r=new Set(e.map(e=>t.rungOf(nr(e.dist/this.camera.frameK,e.lat))).filter(e=>e!==n));return r.size?r:null}ready={plan:e=>{let t=this.posesOf(e),n=/* @__PURE__ */ new Set;for(let e of this.planners())for(let r of e.plan(t))n.add(r);return{needed:[...n]}},coverage:async(e,t={})=>{let n=await this.loader.coverage(this.ready.plan(e).needed,t.scope);return{needed:n.total,have:n.have,missing:[...n.missing,...n.absent],fraction:n.fraction}},ensure:(e,t={})=>this.loader.ensure(this.ready.plan(e).needed,t),rehearse:async(e,t={})=>{let n=this.camera.pose,r=[];for(let n of e){let e=this.nowReal();this.go(n);let i=await this.settled({timeout:t.timeout??6e4}),a=this.report();r.push({view:typeof n==`string`?n:`${n.lon.toFixed(4)},${n.lat.toFixed(4)}`,settled:i,ms:Math.round(this.nowReal()-e),frames:a.frames,pending:a.pending,missing:a.missing})}return this.camera.jump(n),this.invalidate(),r}};gov=null;governorOpts;resolutionFloor=1.5;ratioCaps=[];shadowDrop=0;detailNow=1;qualityDrop=!1;remote=null;heartbeat=null;gaps=[];get governor(){return this.gov?{budget:{...this.gov.budget},dropped:this.gov.drops}:null}setBudget(e){if(!e)return this.gov?.reset(),this.gov=null,this.stateCache=null,this.invalidate(),this;if(this.gov)return this.gov.budget={...e},this;let t=this.governorOpts;return this.gov=new Xa({...e},t===!1?{ladder:[]}:t??{},this.rungs()),this}prio={};setPriorities(e){this.prio={...e};for(let t of this.elements)t.detailPinned=!!e.keep?.includes(t.id);return this.gov?.setLadder(this.rungs()),this.stateCache=null,this.invalidate(),this}log(e,t={}){this.remote?.send(e,t,`info`)}rungs(){let e=this.renderer,t=()=>{this.stateCache=null,this.invalidate()},n=this.resolutionFloor,r=this.worldRungs(e,t,n),i=this.prio.drop??[],a=new Set(this.prio.keep??[]),o=new Map(r.map(e=>[e.name,e]));return[...i.filter(e=>!a.has(e)).map(e=>o.get(e)??this.elementRung(e,t)),...r.filter(e=>!i.includes(e.name)&&!a.has(e.name))]}elementRung(e,t){return{name:e,down:()=>{let n=this.elements.find(t=>t.id===e);return!n||n.detailPinned||n.detailOwn<=.25?null:(n.detailOwn-=.25,t(),`${e} ${n.detailOwn}`)},up:()=>{let n=this.elements.find(t=>t.id===e);n&&(n.detailOwn=Math.min(1,n.detailOwn+.25),t())}}}worldRungs(e,t,n){return[{name:`detail`,down:()=>this.detailNow<=.5?null:(this.detailNow-=.25,t(),`detail ${this.detailNow}`),up:()=>{this.detailNow=Math.min(1,this.detailNow+.25),t()}},{name:`shadows`,down:()=>!this.shadowOpts||this.shadowDrop>=3?null:(this.shadowDrop++,t(),this.shadowDrop>=3?`shadows off`:`shadows ${this.shadowDrop===1?`half`:`quarter`}`),up:()=>{this.shadowDrop=Math.max(0,this.shadowDrop-1),t()}},{name:`quality`,down:()=>this.qualityDrop||this.state.render.quality===`lite`?null:(this.qualityDrop=!0,t(),`quality lite`),up:()=>{this.qualityDrop=!1,t()}},{name:`resolution`,down:()=>{if(!e)return null;let r=e.fullRatio,i=Math.min(n,r);if(r<=i+.01)return null;let a=Math.max(i,Math.floor((r-.5)*4)/4);return this.ratioCaps.push(e.cap),e.setCap(a),t(),`resolution ${a}`},up:()=>{e?.setCap(this.ratioCaps.pop()??null),t()}}]}sample(e){this.recentGaps.push(e),this.recentGaps.length>120&&this.recentGaps.shift(),this.remote?.level===`info`&&(this.gaps.push(e),this.gaps.length>240&&this.gaps.splice(0,120));let t=this.gov;if(!t)return;let n=t.budget,r=this.graphics,i=0;if(n.triangles!==void 0)for(let e of this.lastDraws)i+=e.count*(e.instanceCount??1)/3;let a=t.frame({gapMs:e,cpuMs:this.renderer?.cpuMs??0,gpuMs:r?.stats.gpuMs??null,draws:r?.stats.draws??0,triangles:i,gpuMB:n.memoryMB===void 0?0:(r?.bytes??0)/1048576});a&&(this.emit(`governor`,a),this.remote?.send(`governor`,{...a},`warn`))}beat(){if(this.disposed||!this.remote)return;let e=this.gaps.splice(0).sort((e,t)=>e-t),t=t=>e.length?Math.round(e[Math.min(e.length-1,Math.floor(e.length*t))]):0,n=this.stats(),r=this.frameProfile(),i=n.tiles;this.remote.send(`hb`,{api:this.api,fps:Math.round(n.fps),zoom:+n.zoom.toFixed(2),p50:t(.5),p95:t(.95),max:t(1),long:e.filter(e=>e>33.4).length,draws:r.draws,gpuMs:r.gpuMs,size:r.size,pr:r.pixelRatio,gpuMB:Math.round(n.gpuBytes/1048576),tiles:i?.visible,loading:i?.loading,dropped:this.gov?.drops??[]})}themeS=new ho;subjectBlock=null;subjectData=/* @__PURE__ */ new Float32Array(112);subjectDraws=/* @__PURE__ */ new Set;setTheme(e,t={}){return this.themeS.set(e,t.seconds),this.invalidate(),this}get theme(){return this.themeS.asked}get themeNow(){return this.themeS.shownTheme?this.themeS:null}subjectFrame(e){let t=this.subjectData;return t.set(e),this.themeS.write(t,100,0),this.subjectBlock??=this.graphics.buffer(`uniform`,448,`frame (a theme's subjects)`),this.subjectBlock.write(t),this.subjectBlock}shadowSig=``;progNos=/* @__PURE__ */ new WeakMap;progN=0;progNo(e){let t=this.progNos.get(e);return t===void 0&&(t=++this.progN,this.progNos.set(e,t)),t}constructor(e){this.body=e.body,this.camera=e.camera,this.loader=e.loader,this.ownLoader=e.ownLoader,this.graphics=e.graphics,this.profile=e.profile,this.api=e.graphics?.api??`none`,this.reason=e.reason,this.user={selection:[],hover:null,toggles:{},role:`viewer`,...e.opts.user},this.timeIsNow=e.opts.time===void 0||e.opts.time===`now`,this.time=e.opts.time instanceof Date?e.opts.time:/* @__PURE__ */ new Date,this.clockRate=+!!this.timeIsNow,this.zones=this.zoneTracker(e.opts.timeZone),this.errorHandler=e.opts.onError??(e=>console.error(e));let t=e.profile?.touch??sa(),n=e.opts.render?.quality??e.opts.quality;this.qualityFn=typeof n==`function`?n:null,this.groundMarksChosen=e.opts.groundMarks!==void 0,this.groundMarks=e.opts.groundMarks??(((typeof n==`string`&&n!==`auto`?n:void 0)??e.profile?.quality)===`lite`||t?`tiles`:`prisms`),this.context=e.opts.context??`app`,this.tipOpts=e.opts.tooltip;let r=e.opts.render?.shadows;this.shadowOpts=r?r===!0?{}:r:null;for(let[t,n]of Object.entries(e.opts.views??{}))this.addView(t,n);this.renderer=e.graphics&&e.canvas?new la(e.canvas,e.graphics,{maxPixelRatio:e.profile?.maxPixelRatio??2,pixelRatio:e.opts.pixelRatio,clock:e.opts.clock,tick:e=>this.tick(e),onError:e=>this.error(e),onLong:e=>{if(this.ev.long?.size&&this.emit(`long`,{gapMs:e,profile:this.frameProfile()}),this.remote){let t=this.frameProfile();this.remote.send(`long`,{gapMs:Math.round(e),tickMs:+t.tickMs.toFixed(1),submitMs:+t.submitMs.toFixed(1),elements:t.elements.slice(0,3),draws:t.draws,gpuMs:t.gpuMs},e>100?`warn`:`info`)}},onFrame:e=>this.sample(e),needsFrame:e.opts.renderOnChange===!1?void 0:()=>this.needsFrame(),rest:e.opts.frames?.rest!==!1&&{after:e.opts.frames?.rest??500,look:250},nextIn:()=>this.nextIn(),busy:()=>this.preparing,restNow:()=>this.restNow(),moving:()=>this.camera.moving||this.following!==null,movingPixelRatio:e.opts.resolution?.moving??0,antialias:e.opts.resolution?.antialias??`auto`}):null,this.framesOpts=e.opts.frames??{},this.lastTimeLook=this.nowReal(),this.camera.changed=()=>this.renderer?.wake(),this.camera.under=(e,t,n)=>{let r=this.exaggeration,i=this.standAt(e,t,n/r);return Number.isFinite(i)?i*r:NaN},this.creditsOpts=e.opts.attribution===!1?null:e.opts.attribution??{};let i=e.opts.log;i&&(this.remote=new Za(typeof i==`string`?{url:i}:i)),this.remote?.level===`info`&&(this.heartbeat=setInterval(()=>this.beat(),1e3)),this.resolutionFloor=e.opts.resolution?.floor??1.5;let a=e.opts.budget??`auto`,o=a===`auto`?e.profile?.budget??null:a===!1?null:a;if(this.governorOpts=e.opts.governor,this.prio=e.opts.priorities??{},o&&this.setBudget(o),e.opts.theme&&this.themeS.set(e.opts.theme,0),e.graphics&&e.graphics.lost.then(e=>{this.disposed||this.error(new K(`E-GPU-004`,`${this.api===`webgpu`?`WebGPU`:`WebGL2`}: ${e}`,{fix:`reload the page`}))}),e.canvas&&`addEventListener`in e.canvas&&e.opts.controls!==!1&&typeof window<`u`){let t=e.canvas;this.canvasEl=t,typeof e.opts.controls==`object`&&(this.camera.interrupt=e.opts.controls.interrupt??!0);let n=!1,r=this.camera.attach(t,(e,t)=>{this.tapAt(e,t,n).catch(e=>{this.disposed||this.error(e)})}),i=e=>{n=e.pointerType!==`mouse`,this.movedAt=this.nowReal(),this.renderer?.wake()},a=n=>{if(n.buttons!==0&&(this.movedAt=this.nowReal(),this.renderer?.wake()),n.pointerType!==`mouse`||n.buttons!==0||e.opts.hover===!1)return;this.renderer?.wake();let r=t.getBoundingClientRect();this.pointer={x:n.clientX-r.left,y:n.clientY-r.top,cx:n.clientX,cy:n.clientY},this.hoverWanted=!0},o=()=>{this.pointer=null,this.hoverWanted=!1,this.setHover(null,null).catch(()=>{})};t.addEventListener(`pointerdown`,i),t.addEventListener(`pointermove`,a),t.addEventListener(`pointerleave`,o),this.detachControls=()=>{r(),t.removeEventListener(`pointerdown`,i),t.removeEventListener(`pointermove`,a),t.removeEventListener(`pointerleave`,o)}}}get graphicsApi(){return this.api}get groundVersion(){let e=0;for(let t of this.elements){let n=t.groundVersion;typeof n==`number`&&(e+=n)}return e}get exaggeration(){return this.body?.exaggeration?.(this.state)??1}heightAt(e,t){if(this.grounds===null){this.grounds=[];for(let e of this.elements){let t=e.heightAt;t&&this.grounds.push(t.bind(e))}}for(let n=0;n<this.grounds.length;n++){let r=this.grounds[n](e,t);if(!Number.isNaN(r))return r}return NaN}grounds=null;standAt(e,t,n=NaN){if(this.stands===null){this.stands=[];for(let e of this.elements){let t=e.levelsAt;t&&this.stands.push(t.bind(e))}}let r=this.heightAt(e,t);if(!this.stands.length)return r;let i=this.standLevels;i.length=0;for(let n=0;n<this.stands.length;n++)this.stands[n](e,t,i);return io(i,r,n)}stands=null;standLevels=[];get standVersion(){let e=this.groundVersion;for(let t of this.elements){let n=t.standVersion;typeof n==`number`&&(e+=n)}return e}rangeIn(e,t,n,r){for(let i of this.elements){let a=i.rangeIn;if(a){let o=a.call(i,e,t,n,r);if(o)return o}}return null}add(...e){this.alive();for(let t of e){if(!t||typeof t.frame!=`function`)throw new K(`E-WORLD-001`,`w.add() takes elements`,{fix:`pass points({ … }), layer({ … }), custom({ … }) or your own Element`});if(this.elements.some(e=>e.id===t.id))throw new K(`E-WORLD-001`,`two elements with id '${t.id}'`,{where:t.id,fix:`give each element its own id`});let e=t.props.when;!this.levelsObj&&e&&typeof e==`object`&&e.level!==void 0&&this.observe({levels:`default`}),t.added=this.seq++,t.detailPinned=!!this.prio.keep?.includes(t.id);let n=new Set(this.elements.map(e=>e.pickNo));t.pickNo=0;for(let e=1;e<256;e++)if(!n.has(e)){t.pickNo=e;break}this.elements.push(t);for(let[e,n]of Object.entries(t.params()))this.paramDefs.set(`${t.id}.${e}`,n);for(let[e,n]of Object.entries(t.readouts()))this.readDefs.set(`${t.id}.${e}`,n);for(let[e,n]of Object.entries(t.points()))this.pointDefs.set(`${t.id}.${e}`,n);let r=t.attach({graphics:this.graphics,loader:this.loader,profile:this.profile,camera:this.camera,world:this}).catch(e=>{t.failure=e instanceof Error?e:Error(String(e)),this.error(e)}).finally(()=>{this.loading.delete(r),this.disposed||this.invalidate()});this.loading.add(r);let i=t.companions?.()??[];i.length&&this.add(...i)}return this.resort(),this.invalidate(),this.emit(`elements`,{what:`add`,ids:e.map(e=>e.id)}),this}remove(e){let t=this.element(e);for(let t of this.elements)if(t.above===e||t.below===e)throw new K(`E-WORLD-002`,`'${t.id}' is placed ${t.above===e?`above`:`below`} '${e}'`,{where:e,fix:`move '${t.id}' first`});this.elements=this.elements.filter(e=>e!==t);for(let t of[this.paramDefs,this.readDefs,this.pointDefs])for(let n of[...t.keys()])n.startsWith(`${e}.`)&&t.delete(n);t.detach(),this.gates?.forget(t),this.resort();for(let e of t.companions?.()??[])this.elements.includes(e)&&this.remove(e.id);return this.invalidate(),this.emit(`elements`,{what:`remove`,ids:[e]}),this}get(e){return e.includes(`.`)?this.paramOf(e).get():this.element(e)}element(e){let t=this.elements.find(t=>t.id===e);if(!t)throw new K(`E-WORLD-002`,`no element '${e}'`,{fix:`the elements: ${this.elements.map(e=>e.id).join(`, `)||`none`}`});return t}get list(){return this.sorted}get level(){return this.levelsObj?.level??null}get levels(){return this.levelsObj?.ladder??null}get observation(){let e=this.levelsObj,t=this.gates;return!e||!t?null:{level:e.level,by:e.by,ladder:e.ladder,margin:e.margin,dwell:e.dwell,ahead:e.ahead,keep:e.keep,waiting:e.waiting!==null,swapping:t.swap!==null,counts:t.counts}}observe(e){return e===null?(this.levelsObj=null,this.gates=null):(this.levelsObj=new Et(e,this.camera.zoom,(this.levelsObj,void 0)),this.gates=new Ya(this.levelsObj)),this.stateCache=null,this.invalidate(),this}setLevel(e){this.levelsObj||this.observe({levels:`default`});let t=this.levelsObj.set(e);return t&&this.levelChanged(t),this}levelChanged(e){this.stateCache=null,this.gates.begin(e,this.sorted,this.state,this.nowMs),this.invalidate(),this.emit(`level`,e)}prepCtx(e,t){return{graphics:this.graphics,camera:this.camera,s:e,frame:this.renderer?.frameBlock,now:this.nowMs,loadNow:this.nowMs+this.holdMs,dt:t,world:this}}observeStep(){let e=this.levelsObj;if(!e)return;let t=e.step(this.camera.zoom,this.nowMs,typeof e.bind==`function`?this.state:void 0);t&&this.levelChanged(t)}elementsOfKind(e){return this.sorted.filter(t=>t.kind===e)}move(e,t){return this.element(e).setProps({above:t.above,below:t.below,order:t.order}),this.resort(),this}reorder(e){let t=e.map(e=>this.element(e)),n=e=>{if(e.sits===`above`&&(e.kind===`area`||e.kind===`line`))return 4.5;let t=ua[e.kind];return t>=1&&t<4?2:t};return t.forEach((e,t)=>{e.props={...e.props,order:n(e)+(t+1)/1e3,above:void 0,below:void 0},e.restacked?.((t+1)/1e3)}),this.resort(),this.invalidate(),this.emit(`elements`,{what:`order`,ids:t.map(e=>e.id)}),this}setOpacity(e,t){let n=this.element(e);if(!n.fades)throw new K(`E-WORLD-004`,`layer '${e}' cannot be made see-through`,{where:e,fix:`only marks drawn from data and pictures fade (layer.fades)`});return n.setProps({opacity:Math.max(0,Math.min(1,t))}),this}ownedOnce(e,t){if(t!==void 0)for(let n of this.elements)n.owner===e&&(n.once=t)}elementChanged(e,t){this.elements.includes(e)&&(t.some(e=>e===`order`||e===`above`||e===`below`||e===`drape`||e===`over`||e===`occlude`)&&this.resort(),this.emit(`elements`,{what:t.length===1&&t[0]===`opacity`?`opacity`:`props`,ids:[e.id]}))}resort(){this.sorted=da(this.elements),this.grounds=null,this.stands=null}param(e,t,n,r){if(!/^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+$/.test(e))throw new K(`E-WORLD-004`,`param key '${e}' must be dotted, like 'fault.glow'`,{fix:`write <group>.<name>`});let i=t.default;return this.paramDefs.set(e,{...t,get:r??(()=>i),set:e=>{i=e,n(e)}}),this}paramOf(e){let t=this.paramDefs.get(e);if(!t)throw new K(`E-WORLD-004`,`no param '${e}'`,{fix:`the params: ${[...this.paramDefs.keys()].join(`, `)||`none`}`});return t}set(e,t){return this.paramOf(e).set(t),this.stateCache=null,this.invalidate(),this}params(){let e={};for(let[t,n]of this.paramDefs)e[t]={type:n.type,default:n.default,range:n.range,options:n.options,unit:n.unit,describe:n.describe,value:n.get()};return e}readout(e,t){return this.readDefs.set(e,t),this}read(e){let t=this.readDefs.get(e);if(!t)throw new K(`E-WORLD-004`,`no readout '${e}'`,{fix:`the readouts: ${[...this.readDefs.keys()].join(`, `)||`none`}`});return t()}point(e,t){return this.pointDefs.set(e,t),this}pointAt(e){return this.pointDefs.get(e)?.()??null}addView(e,t){return pr(t,`view '${e}'`),this.views.set(e,t),this}resolve(e){if(typeof e==`string`){let t=this.views.get(e);if(!t)throw new K(`E-VIEW-001`,`no view '${e}'`,{fix:`the views: ${[...this.views.keys()].join(`, `)||`none`}`});return t}return pr(e),e}go(e){return this.camera.jump(this.resolve(e)),this}fly(e,t){let n=this.camera.fly(this.resolve(e),t);return(t?.seconds??1)>=1&&!this.recordingNow&&this.sorted.some(e=>typeof e.pumpAhead==`function`)&&this.prefetch([e],{timeout:((t?.seconds??6)+2)*1e3}),n}play(e,t={}){if(!(`aim`in e)){let n=this.camera.play(e,t);if(e.duration>=1&&!this.recordingNow&&this.aheadAble()){let t=e.list[0].at;this.prefetch(Array.from({length:9},(n,r)=>gr(e.pose(t+e.duration*r/8))),{timeout:(e.duration+2)*1e3})}return n}let n=this.shot,r=new ti(e,t,n&&n.sameAim(e)?n.follower:void 0);this.shot=r,this.following=null;let i=this.moveWorld(),a=this.camera,o=a.drive(e=>{let t=r.step(e,i,this.camInfo());return t&&a.place(t.pose,t.ground),r.done});if((r.seconds??0)>=1&&!this.recordingNow&&this.aheadAble()){let e=r.seconds,t=this.clockRate,n=this.timeMs,a=Math.max(2,Math.min(12,Math.ceil(e))),o=Array.from({length:a+1},(o,s)=>r.poseAt(e*s/a,i,this.camInfo(),n+e*s/a*t*1e3)).filter(e=>!!e);o.length&&this.prefetch(o,{timeout:(e+2)*1e3})}return o}get playing(){return this.camera.driving&&this.shot?this.shot.move:null}moveWorld(){let e=this;return{heightAt:(t,n)=>e.heightAt(t,n),get exaggeration(){return e.exaggeration},pointAt:t=>e.pointAt(t),get timeMs(){return e.timeMs},topIn:(t,n,r,i)=>e.topIn(t,n,r,i)}}camInfo(){let e=this.camera;return{pose:e.pose,groundY:e.groundY,fov:e.fov,floor:t=>e.pitchAt(t)}}aheadAble(){return this.sorted.some(e=>typeof e.pumpAhead==`function`)}viewToFit(e,t={}){let n=Array.isArray(e)&&typeof e[0]==`number`?e:(Array.isArray(e)?e:[e]).reduce((e,t)=>O(t.geometry,e??void 0)??e,null);if(!n||!Number.isFinite(n[0]))throw new K(`E-VIEW-001`,`nothing to fit: the box or features have no extent`,{fix:`fit a [w, s, e, n] box or features with geometry`});for(let e of[`padding`,`maxZoom`]){let n=t[e];if(n!==void 0&&!(typeof n==`number`&&Number.isFinite(n)))throw new K(`E-VIEW-001`,`fit: ${e} is ${n}`,{where:`fit`,fix:`give ${e} as a finite number, or leave it out`})}let r=Math.max(-85,Math.min(85,(n[1]+n[3])/2)),i=(n[0]+n[2])/2,a=this.camera,o=Math.max(1,(n[2]-n[0])*111320*Math.cos(r*G)),s=Math.max(1,(n[3]-n[1])*110574),c=Math.max(.2,a.width/Math.max(1,a.height)),l=1+2*(t.padding??.08),u=Math.max(s,o/c)*l/(2*Math.tan(a.fov/2));return t.maxZoom!==void 0&&(u=Math.max(u,rr(t.maxZoom,r)*a.frameK)),{lon:i,lat:r,zoom:nr(u/a.frameK,r),tilt:t.tilt??0,bearing:t.bearing??0}}fit(e,t={}){return this.camera.fly(this.viewToFit(e,t),t)}follow(e,t={}){if(t.smooth!==void 0&&!(Number.isFinite(t.smooth)&&t.smooth>=0))throw new K(`E-VIEW-001`,`follow: smooth is ${t.smooth}`,{where:`follow`,fix:`give smooth in ms, a finite number 0 or more (default 250), or leave it out`});if(t.dist!==void 0&&!(Number.isFinite(t.dist)&&t.dist>0))throw new K(`E-VIEW-001`,`follow: dist is ${t.dist}`,{where:`follow`,fix:`give dist in metres above 0, or leave it out`});if(typeof t.heading==`number`&&!Number.isFinite(t.heading))throw new K(`E-VIEW-001`,`follow: heading is ${t.heading}`,{where:`follow`,fix:`give heading as degrees from straight behind it (a finite number), true, or leave it out`});if(Or(t.damping,`follow`),e===null)return this.following=null,this;Rr(e,`follow`);let n=t.damping??(t.smooth===void 0?{}:{place:{halfLife:.84*t.smooth/1e3,lead:0}}),r=t.heading===void 0||t.heading===!1?null:t.heading===!0?0:t.heading,i=this.camera,a=this.following;return a?(a.target=e,a.f.damping=n,a.heading=r,a.dist=t.dist??null,this):(i.driving&&i.stop(),this.following={target:e,heading:r,dist:t.dist??null,f:new Fr(n),hands:{...i.hands},eye:i.dist,up:0,seen:!1,last:null},this)}followStep(e){let t=this.following,n=this.camera,r=n.hands;if(r.pan!==t.hands.pan||t.heading!==null&&r.turn!==t.hands.turn||t.dist!==null&&r.zoom!==t.hands.zoom){this.following=null,n.held=!1;return}r.zoom!==t.hands.zoom&&(t.eye=Math.max(.01,n.dist-t.up/Math.max(.05,Math.sin(n.pitch))),t.hands.zoom=r.zoom),t.hands.turn=r.turn;let i=zr(t.target,this,this.timeMs);if(!i)return;t.last=[i.lon,i.lat,i.y],t.seen||(t.seen=!0,t.eye=n.dist,t.f.seed({place:[n.lon,n.lat,i.y],dist:t.dist===null?void 0:n.dist,heading:t.heading===null?void 0:n.pose.bearing-t.heading}));let a=t.f.step({place:[i.lon,i.lat,i.y],heading:t.heading===null?void 0:Number.isFinite(i.heading)?i.heading:null,dist:t.dist??void 0},e/1e3,t.eye),o=a.place,s=a.dist??t.eye,c=o[2]-i.g,l=a.heading===void 0?n.pose.bearing:a.heading+t.heading;if(i.y-i.g<.05&&a.dist===void 0)n.lon=Y(o[0]),n.lat=o[1],a.heading!==void 0&&(n.yaw=-l*G);else{let e=this.exaggeration,t=Ur({lon:o[0],lat:o[1],y:o[2]},{dist:s,tilt:90-n.pitch/G,bearing:l},(t,n)=>{let r=this.heightAt(t,n);return Number.isFinite(r)?r*e:i.g},n.fov,e=>n.pitchAt(e));n.place(t.pose,t.ground)}t.eye=s,t.up=Math.max(0,c),n.held=!0}get isFollowing(){return this.following!==null}controlStop=null;camLimits=null;control(e,t,n){let r=!e&&t&&`gestures`in t?t:null;for(let[e,t]of[[`zoomRange`,r?.zoomRange],[`tiltRange`,r?.tiltRange]])if(t&&!(t.length===2&&t.every(e=>typeof e==`number`&&Number.isFinite(e))))throw new K(`E-VIEW-001`,`control: ${e} is ${JSON.stringify(t)}`,{where:`control`,fix:`give ${e} as two finite numbers [${e===`zoomRange`?`out, in`:`least, most`}], or leave it out`});this.controlStop?.(),this.controlStop=null;let i=this.camera;if(this.camLimits??={minDist:i.minDist,maxDist:i.maxDist},e){let r=e.layer;if(!r?.control)throw new K(`E-WORLD-005`,`entity '${e.id}' is in no entity layer`,{fix:`add it to an entityElement, and that layer to the world, first`});return this.controlStop=r.control(this,e,t,n),this}i.gestures={...r?.gestures??{pan:!0,rotate:!0,tilt:!0,zoom:!0}},i.drag=r?.drag??`pan`;let a=r?.zoomRange,o=r?.tiltRange;return i.minDist=a?rr(a[1],i.lat)*i.frameK:this.camLimits.minDist,i.maxDist=a?rr(a[0],i.lat)*i.frameK:this.camLimits.maxDist,i.zoomLimited=!!a,i.pitchRange=o?[(90-Math.min(88,o[1]))*G,Math.min(Math.PI/2-1e-4,(90-o[0])*G)]:[2*G,Math.PI/2-1e-4],i.dist=Math.min(i.maxDist,Math.max(i.minDist,i.dist)),i.pitch=Math.min(i.pitchRange[1],Math.max(i.pitchRange[0],i.pitch)),this.invalidate(),this}setProjection(e,t=1.2){return this.camera.setProjection(e,t),this.stateCache=null,this}project(e){let t=typeof e==`string`?this.pointAt(e):e;if(!t)return null;let n=this.groundOf(t);return this.camera.facing(t[0],t[1],n)?this.camera.project(t[0],t[1],n):null}visible(e){let t=typeof e==`string`?this.pointAt(e):e;if(!t)return!1;let n=this.groundOf(t),r=this.project(t);if(!r||r[0]<0||r[1]<0||r[0]>this.camera.width||r[1]>this.camera.height)return!1;let i=this.camera,a=we(i,t[0],t[1],n),o=i.eye,s=this.exaggeration;for(let e=1;e<24;e++){let t=e/24,n=a[0]+(o[0]-a[0])*t,r=a[1]+(o[1]-a[1])*t,c=a[2]+(o[2]-a[2])*t,l=i.morph>.5?[i.lon+n/(J*Math.cos(i.lat*G))/G,i.lat-c/J/G]:null;if(!l)break;let u=this.heightAt(l[0],l[1])*s;if(!Number.isNaN(u)&&u>r+2)return!1}return!0}groundOf(e){if(e[2]!==void 0)return e[2]*this.exaggeration;let t=this.heightAt(e[0],e[1]);return Number.isNaN(t)?0:t*this.exaggeration}get state(){if(this.stateCache)return this.stateCache;let e=this.camera,t=e.view,n={},r={};for(let[e,t]of this.readDefs)Object.defineProperty(n,e,{get:t,enumerable:!0});for(let[e,t]of this.paramDefs)Object.defineProperty(r,e,{get:()=>t.get(),enumerable:!0});let i=ir({observation:this.levelsObj?.state??et,view:{lon:t.lon,lat:t.lat,zoom:e.zoom,dist:e.dist,tilt:t.tilt,bearing:t.bearing,altitude:e.dist*Math.sin(e.pitch),pixelSize:e.pixelSize,width:e.width,height:e.height,morph:e.morph},params:r,readouts:n,user:this.user,device:{api:this.api,tier:this.profile?.tier??`mid`,reason:this.reason},quality:this.qualityDrop?`lite`:this.qualityNow??this.profile?.quality??`realistic`,detail:this.detailNow});if(this.qualityFn&&!this.qualityDrop&&this.qualityNow===null)try{let e=this.qualityFn(i);(e===`lite`||e===`realistic`)&&(i.render.quality=e)}catch(e){this.error(e)}return i.time=er(this.time,t.lon,t.lat,this.nowMs/1e3),i.time.rate=this.clockRate,i.time.local=qn(this.time,t.lon,this.zones.zoneAt(t.lon,t.lat)),this.stateCache=i,i}get timeRunning(){return this.timeIsNow}get now(){return this.nowMs}tick(e){this.nowMs+=e,this.frameN++,this.dirtySincePick=!0;let t=this.nowReal();if(this.drawnAt=t,this.lastTimeLook=t,this.timeIsNow&&this.renderer?.isRunning&&typeof this.renderer.clock!=`object`)this.time=/* @__PURE__ */ new Date,this.timeRest=0;else if(this.timeIsNow||this.clockRate){let t=this.time.getTime()+this.timeRest+e*(this.timeIsNow?1:this.clockRate),n=Math.floor(t);this.time=new Date(n),this.timeRest=t-n}let n=this.camera,r=this.renderer;this.befores.length&&(this.stateCache=null,this.runEvery(this.befores,this.state,e)),this.following&&this.followStep(e),n.held=this.following!==null||n.driving,n.update(r?r.cssWidth:n.width,r?r.cssHeight:n.height,e);let i=this.heightAt(n.lon,n.lat),o=this.exaggeration;if(Number.isNaN(i)||(this.groundTarget=i*o,n.groundY+=(this.groundTarget-n.groundY)*Math.min(1,e/250),Math.abs(this.groundTarget-n.groundY)<this.groundTol()&&(n.groundY=this.groundTarget)),n.morph>.5){let e=n.eye,t=a(n,e[0],e[1],e[2]),r=this.heightAt(t[0],t[1])*o;n.clearance=Math.max(0,e[1]-(Number.isNaN(r)?n.groundY:Math.max(n.groundY,r)))}else n.clearance=0;this.poseCheck(),this.stateCache=null,this.observeStep();let s=this.state,c=this.gates;c&&(c.keep(this.nowMs),c.work(this.prepCtx(s,e),4,this.nowMs,()=>this.nowReal()));let l=null;for(let t of this.sorted){let n=t.target;try{t.target=t.forced===null?j(t.props.when,s,`element '${t.id}'`):+!!t.forced}catch(e){t.target=0,this.error(e)}let r=c?.hold(t);r!==void 0&&t.forced===null&&(t.target=r),t.owner&&(t.target=Math.min(t.owner.forced===!1?0:t.owner.target,t.target)),n>0!=t.target>0&&(l??=[]).push(t.id);let i=t.once??t.revealSeconds(),a=i>0?e/1e3/i:1;t.shown=t.shown<t.target?Math.min(t.target,t.shown+a):Math.max(t.target,t.shown-a),t.shown===t.target&&(t.once=null)}l&&this.emit(`elements`,{what:`shown`,ids:l});let u=[];if(r){vt();let t=r.frameData,i=n.eye;t.set(n.viewProj,0),t.set([i[0],i[1],i[2],this.nowMs/1e3],16),t.set([this.graphics.width,this.graphics.height,r.pixelRatio,n.morph],20);let a=this.lightNow=this.lightOf(s);t.set(a?[a.dir[0],a.dir[1],a.dir[2],a.daylight]:[.4,.8,.45,1],24),t.set([n.lat*G,Math.cos(n.lat*G),Math.tan(n.fov/2),J],28);let o=a?.clouds&&(a.cover??0)>.01?a.clouds:null;t.set([q(Math.max(-85.0511,Math.min(85.0511,n.lat))),Math.sin(n.lat*G),o?o.x:0,o?o.z:0],32),t.set([0,0,o?o.base:0,0],36),t.set([1,1,0,0],40),t.set(n.inv,44);let c=this.shadowsFor(a);t[60]=this.graphics.width,t[61]=this.graphics.height,t[62]=c?.8*c.texel*c.depthPerMetre:0,t[63]=n.height,t.set(c?c.viewProj:go,64),t.set(a?[a.color[0],a.color[1],a.color[2],1]:[0,0,0,0],80),t.set(a?[a.ambient[0],a.ambient[1],a.ambient[2],a.bounce]:[0,0,0,0],84),t.set(c?[this.shadowOpts?.strength??1,1/c.size,1.5*c.texel,.25*c.depthPerMetre+1.5/16777216]:[0,0,0,0],88),t.set(a?.haze?[a.haze[0],a.haze[1],a.haze[2],a.visibility??0]:[0,0,0,0],92),t.set([a?.wet??0,a?.flash??0,a?.cover??0,a?.wind??0],96);let l=this.themeS;l.step(e,s),l.write(t,100),r.frameBlock.write(t);let d={graphics:this.graphics,camera:n,s,frame:r.frameBlock,now:this.nowMs,n:this.frameN,loadNow:this.nowMs+this.holdMs,dt:e,world:this,shadowMap:(c?this.shadowTarget?.texture:null)??this.blankShadow(),shadowView:c?c.viewProj:null,sea:this.seaOf(s)},f=null;this.subjectDraws.clear(),this.drawsOf.clear();let p=this.layerMs;p.clear(),d.labelRoom=[];for(let e of this.labelsTopFirst()){if(e.shown<=0&&e.target<=0&&e.kind!==`basemap`&&e.kind!==`sky`)continue;let n=this.nowReal(),r=l.active&&l.subject(e);try{let n=e.frame(r?f??={...d,frame:this.subjectFrame(t)}:d);if(this.drawsOf.set(e,n),u.push(...n),r)for(let e of n)this.subjectDraws.add(e)}catch(e){this.error(e)}p.set(e.id,this.nowReal()-n)}if(c?this.drawShadows(u,t,c):this.shadowStats.on=!1,u.some(e=>e.shadowOnly)){let e=u.filter(e=>!e.shadowOnly);u.length=0,u.push(...e);for(let[e,t]of this.drawsOf)t.some(e=>e.shadowOnly)&&this.drawsOf.set(e,t.filter(e=>!e.shadowOnly))}}this.lastDraws=u,this.runEvery(this.everies.filter(e=>!e.before),s,e),this.creditsOpts&&this.credits(),r&&this.prepareSoon({graphics:this.graphics,camera:n,s,frame:r.frameBlock,now:this.nowMs,loadNow:this.nowMs+this.holdMs,dt:e,world:this},3);let d=u,f=this.compareView,p=/* @__PURE__ */ new Set,m=/* @__PURE__ */ new Set,h=/* @__PURE__ */ new Set;if(f){for(let e of f.leftHides)for(let t of this.drawsOf.get(e)??[])m.add(t),p.add(t);for(let e of f.rightHides)for(let t of this.drawsOf.get(e)??[])h.add(t),p.add(t)}if(r){let t=this.groundMarks===`tiles`?this.sorted.find(e=>e.kind===`basemap`&&typeof e.overlays==`function`&&(this.drawsOf.get(e)?.length??0)>0):void 0;if(t){let i=u.filter(e=>e.ground&&!p.has(e)),a=typeof r.clock==`object`,o=this.themeS.key(),c=o?i.map(e=>this.subjectDraws.has(e)?e.ground:{...e.ground,version:e.ground.version+o,since:e.ground.since&&{...e.ground.since,version:e.ground.since.version+o}}):i.map(e=>e.ground);try{t.overlays(c,{graphics:this.graphics,camera:n,s,frame:r.frameBlock,now:this.nowMs,loadNow:this.nowMs+this.holdMs,dt:e,world:this},r.pixelRatio,a)&&i.length&&(d=u.filter(e=>!e.ground||p.has(e)))}catch(e){this.error(e)}}}return this.standingNow=this.standsOver(),d.some(e=>e.upper)&&(d=this.banded(d)),this.renderer&&(this.renderer.split=f?{at:f.at,left:d.filter(e=>!m.has(e)),right:d.filter(e=>!h.has(e))}:null),this.emit(`frame`,{n:this.frameN,dt:e}),n.moving||this.pickSoon(),d}labelsTopFirst(){let e=this.sorted,t=[];for(let n=0;n<e.length;n++)e[n].kind===`label`&&t.push(n);if(t.length<2)return e;let n=e.slice();for(let r=0;r<t.length;r++)n[t[r]]=e[t[t.length-1-r]];return n}standingNow=!1;get standing(){return this.standingNow}standsOver(){for(let[e,t]of this.drawsOf)if(!(e.kind===`basemap`||e.kind===`sky`||e.shown<=0&&e.target<=0))for(let e of t){let t=e.program?.desc;if(t&&!e.shadowOnly&&!t.overlay&&!t.sceneDepth&&(t.depth?.write??!0))return!0}return!1}topIn(e,t,n,r){let i=null;for(let a of this.sorted){let o=a.topIn;if(!o||a.shown<=0&&a.target<=0)continue;let s=o.call(a,e,t,n,r);s!==null&&(i===null||s>i)&&(i=s)}return i}banded(e){let t=/* @__PURE__ */ new Set,n=/* @__PURE__ */ new Set,r=!1;for(let e of this.sorted){let i=this.drawsOf.get(e)??[];if(e.kind===`basemap`||e.kind===`sky`)for(let e of i)t.add(e);if(!r&&(e.order??ua[e.kind])>4&&(r=!0),r)for(let e of i)n.add(e)}let i=/* @__PURE__ */ new Set;for(let[e,t]of this.drawsOf)if(e.stands===!0)for(let e of t)i.add(e);let a=e.filter(e=>e.upper===!0),o=e.filter(e=>e.upper===`ground`),s=[],c=[],l=!1,u=!o.length,d=()=>{u=!0,c.push(...o,...s),s.length=0},f=()=>{u||d(),l=!0,c.push(...a)};for(let t of e)t.upper||(!l&&n.has(t)&&f(),!u&&(t.stands||i.has(t))&&!n.has(t)?s.push(t):c.push(t));l||f();let p=[],m=!1;for(let e of c)!m&&!t.has(e)?(p.push({...e,keepDepth:!0}),m=!0):p.push(e);return p}lightOf(e){for(let t of this.sorted)if(!(!t.light||t.shown<=0&&t.target<=0&&t.kind!==`sky`))try{let n=t.light(e);if(n)return n}catch(e){this.error(e)}return null}seaOf(e){for(let t of this.sorted)if(!(!t.sea||t.shown<=0&&t.target<=0))try{let n=t.sea(e);if(n)return n}catch(e){this.error(e)}return null}get light(){return this.lightNow}setShadows(e){return this.shadowOpts=e?e===!0?{}:e:null,this.invalidate(),this}shadowReport(){return{...this.shadowStats}}shadowsFor(e){let t=this.shadowOpts;if(!t||!e||!e.shadows||e.dir[1]<.03||!this.graphics||this.camera.morph<.98||this.shadowDrop>=3)return null;let n=this.state.render.quality===`lite`,r=Math.max(256,(t.size??(n?1024:2048))>>this.shadowDrop),i=this.camera,a=J*Math.cos(i.lat*G),o=it(i,e.dir,r,t.distance??(n?700:1500),[a*i.lon*G,0,-a*q(Math.max(-85.0511,Math.min(85.0511,i.lat)))]);return o?((!this.shadowTarget||this.shadowTarget.width!==r)&&(this.shadowTarget?.destroy(),this.shadowTarget=this.graphics.target(r,r,{sampled:!0,label:`shadow map`}),this.shadowSig=``),this.shadowStats.radius=o.texel*r/2,{...o,size:r}):null}blankShadow(){return!this.noShadow&&this.graphics&&(this.noShadow=this.graphics.texture({label:`no shadow`,width:1,height:1,format:`rgba8`,filter:`nearest`}),this.noShadow.write(new Uint8Array([255,255,255,255]))),this.noShadow}drawShadows(e,t,n){let r=this.graphics,i=this.nowReal(),a=this.shadowTarget,o=a.texture,s=this.blankShadow(),c=this.shadowData;c.set(t),c.set(n.viewProj,0),c.set([n.eye[0],n.eye[1],n.eye[2],t[19]],16),c.set([n.size,n.size,1,t[23]],20),c.set([0,1,0,0],36),c.set([1,1,0,0],40),c[60]=n.size,c[61]=n.size,this.shadowBuf??=r.buffer(`uniform`,448,`shadow frame`),this.shadowBuf.write(c);let l=[],u=`${n.size}|${Array.prototype.join.call(n.viewProj,`,`)}|${t[24]},${t[25]},${t[26]}`,d=e=>e?`${e.uid}.${e.version}`:`-`;for(let t of e){if(!t.cast||!t.pickProgram||t.bounds&&!Pt(n.viewProj,t.bounds))continue;let e=t.pickProgram();l.push({...t,program:e,blocks:[this.shadowBuf,...t.blocks.slice(1)],textures:t.textures?.map(e=>e===o?s:e),pickProgram:void 0}),u+=`;${this.progNo(e)}:${t.blocks.slice(1).map(d).join(`,`)}:${d(t.instances)}:${d(t.vertices)}:${t.count}:${t.instanceCount??1}:${t.first??0}`}if(u===this.shadowSig&&this.shadowTarget===a)this.shadowStats={...this.shadowStats,on:!0,ms:0,kept:!0};else{this.shadowSig=u;try{r.frame(l,{clear:[1,1,1,1],target:a})}catch(e){this.error(e)}this.shadowStats={on:!0,size:n.size,casters:l.length,ms:+(this.nowReal()-i).toFixed(2),radius:this.shadowStats.radius,kept:!1}}}prepareGround(e,t){return this.sorted.find(e=>e.kind===`basemap`&&`prepareOverlays`in e)?.prepareOverlays?.(e,t,this.renderer?.pixelRatio??1)??!0}soon=[];soonAt=-1e9;soonDone=/* @__PURE__ */ new WeakSet;aheadLevels=null;prepareSoon(e,t){let n=this.nowReal();if(n-this.soonAt>250){this.soonAt=n,this.soon.length=0;let t=e.s,r=this.levelsObj,i=r?r.near(this.camera.zoom):null,a=[];for(let e of/* @__PURE__ */ new Set([...i===null?[]:[i],...this.aheadLevels??[]])){if(!r)break;let n=Object.create(t);Object.defineProperty(n,"observation",{value:r.stateAt(e)}),a.push(n)}for(let e of this.sorted){let t=e.props.when;if(!(e.target>0||e.shown>0||e.forced===!0||this.soonDone.has(e))){if(e.forced===!1)this.soon.push(e);else if(a.length&&t!==void 0&&typeof t!=`boolean`)try{a.some(e=>j(t,e)>0)&&this.soon.push(e)}catch{}}}}for(let r of this.soon){if(this.nowReal()-n>t)break;if(!(this.soonDone.has(r)||r.target>0))try{r.prepare(e)&&this.soonDone.add(r)}catch(e){this.soonDone.add(r),this.error(e)}}this.preparing=this.soon.some(e=>!this.soonDone.has(e)&&e.target<=0)}groundTol(){let e=this.camera;return Math.max(.05,.1*e.pixelSize)}sig=/* @__PURE__ */ new Float64Array(10);idleSince=-1;settledSeen=!1;dirty=!0;poseBad=!1;poseCheck(){let e=this.camera,t=[`lon`,`lat`,`dist`,`yaw`,`pitch`,`groundY`].find(t=>!Number.isFinite(e[t]));t&&!this.poseBad&&this.error(new K(`E-VIEW-001`,`the camera's ${t} is ${e[t]} (written into the camera itself: nothing there checks it)`,{where:`camera`,fix:`set the view with w.go or w.fly, which check it (the camera's own yaw and pitch are radians; the view in degrees is w.camera.view)`})),this.poseBad=t!==void 0}movedAt=-1e9;restNow(){return this.idleSince>=0&&this.settledSeen&&!this.hoverWanted&&!this.picking&&this.nowReal()-this.movedAt>(this.framesOpts.rest===!1?1/0:this.framesOpts.rest??500)}invalidate(){this.dirty=!0,this.renderer?.wake()}needsFrame(){let e=this.camera,t=this.renderer,n=this.nowReal(),r=[e.lon,e.lat,e.dist,e.pitch,e.yaw,e.groundY,e.morph,t.cssWidth,t.cssHeight,t.pixelRatio],i=!1;for(let e=0;e<10;e++)this.sig[e]!==r[e]&&!(Number.isNaN(this.sig[e])&&Number.isNaN(r[e]))&&(i=!0,this.sig[e]=r[e]);if(this.hoverWanted&&!i&&!this.dirty&&!e.moving&&!this.following&&this.pickSoon(),this.poseBad&&!i&&!this.dirty)return!1;let a=i?`view`:this.dirty?`dirty`:e.moving?`moving`:this.followMoves()?`follow`:this.loading.size?`loading`:``;if((a===`view`||a===`moving`||a===`follow`)&&(this.movedAt=n),a)return this.dirty=!1,this.idleSince=-1,this.drawFor(a);if(this.themeS.easing)return this.idleSince=-1,this.drawFor(`theme`);if(this.levelsObj&&(this.levelsObj.waiting!==null||this.gates?.swap))return this.idleSince=-1,this.drawFor(this.gates?.swap?`swap`:`level`);let o=this.wanted(n);if(o.rate>0)return this.idleSince=-1,n-this.drawnAt>=$t(o.rate)-4&&this.drawFor(`animating:${o.who}`);let s=this.framesOpts.light??6e4;if(this.timeIsNow&&n-this.lastTimeLook>s&&this.sorted.some(e=>e.light))return this.lastTimeLook=n,this.stateCache=null,this.drawFor(`time`);if(this.idleSince<0){if(this.settledSeen=this.report().settled,!this.settledSeen)return this.drawFor(`unsettled:${this.report().pending[0]??``}`.slice(0,60));this.idleSince=n}if(n-this.idleSince>250){this.idleSince=n;let e=this.report();if(!e.settled)return this.idleSince=-1,this.drawFor(`look:${e.pending[0]??``}`.slice(0,60));if(n-this.lastWhenLook>1e3){this.lastWhenLook=n;let e=null;for(let t of this.sorted){if(typeof t.props.when!=`function`||t.forced!==null)continue;e??=(this.stateCache=null,this.state);let n=0;try{n=j(t.props.when,e,`layer '${t.id}'`)}catch{n=0}if(n!==t.target)return this.drawFor(`look:when`)}}}return this.graphics&&t&&this.prepareSoon({graphics:this.graphics,camera:e,s:this.state,frame:t.frameBlock,now:this.nowMs,loadNow:this.nowMs+this.holdMs,dt:0,world:this},4),!1}followMoves(){let e=this.following;if(!e)return!1;let t=zr(e.target,this,this.timeMs),n=this.camera;if(!t)return!1;if(!e.last)return!0;let r=Y(t.lon-e.last[0])*111320*Math.cos(n.lat*G),i=(t.lat-e.last[1])*110574;return Math.hypot(r,i,t.y-e.last[2])>.1*n.pixelSize}framesOpts={};drawnAt=-1e9;wantNow={rate:0,who:``};everies=[];everyN=0;befores=[];preparing=!1;wanted(e){let t=this.framesOpts,n=this.state,r=this.wantNow;r.rate=0,r.who=``;let i=(e,t)=>{e>r.rate&&(r.rate=e,r.who=t)};for(let e of this.sorted){if(e.shown<=0&&e.target<=0)continue;let r=0;try{r=e.wantsFrames(n),t.rate&&r>0&&(r=t.rate(e,r,n))}catch(e){this.error(e)}r>0&&i(r,e.id)}for(let e of this.everies)e.fps>0&&i(e.fps,`everyFrame#${e.id}`);return!this.timeIsNow&&this.clockRate!==0&&this.sorted.some(e=>e.light)&&i(hn(this.clockRate),`time-lapse`),t.max!==void 0&&r.rate>t.max&&(r.rate=Math.max(0,t.max)),r}nextIn(){let e=this.nowReal(),t=this.wanted(e);return t.rate>0?Math.max(0,$t(t.rate)-(e-this.drawnAt)):1/0}everyFrame(e,t={}){let n={fn:e,fps:t.fps??1/0,budgetMs:t.budgetMs??1,ms:0,over:0,id:++this.everyN,before:!!t.before};return this.everies.push(n),n.before&&this.befores.push(n),this.invalidate(),()=>{for(let e of[this.everies,this.befores]){let t=e.indexOf(n);t>=0&&e.splice(t,1)}}}runEvery(e,t,n){for(let r of e){let e=this.nowReal();try{r.fn(t,n)}catch(e){this.error(e)}r.ms=this.nowReal()-e,r.ms>r.budgetMs?++r.over===30&&this.remote?.send(`everyFrame`,{id:r.id,ms:+r.ms.toFixed(2),budgetMs:r.budgetMs},`warn`):r.over=0}}drawReasons=/* @__PURE__ */ new Map;lastWhenLook=0;drawFor(e){return this.drawReasons.set(e,(this.drawReasons.get(e)??0)+1),!0}drawnWhy(){let e=Object.fromEntries(this.drawReasons);return this.drawReasons.clear(),e}step(e=1e3/60){return this.renderer?this.renderer.step(e):this.tick(e),this}start(){return this.renderer?.start(),this}stop(){return this.renderer?.stop(),this}report(e={}){let t=[],n=[],r=[];this.loading.size&&t.push(`${this.loading.size} layer(s) attaching`),!e.hold&&this.camera.moving&&t.push(`the camera is moving`),!e.hold&&Math.abs(this.groundTarget-this.camera.groundY)>this.groundTol()&&t.push(`the camera is settling onto the ground`),!e.hold&&this.renderer&&this.themeS.easing&&t.push(`the theme is easing`),!e.hold&&this.levelsObj?.waiting!=null&&t.push(`the observation level is about to change (its dwell)`),!e.hold&&this.gates?.swap&&t.push(`the observation level's swap to '${this.gates.swap.to}' is under way`);for(let i of this.elements){if(!e.hold&&i.shown!==i.target&&t.push(`'${i.id}' is ${i.shown<i.target?`revealing`:`hiding`}`),i.failure){n.push({what:i.id,error:i.failure.message});continue}let a=i.status();t.push(...a.pending.map(e=>`${i.id}: ${e}`)),n.push(...a.failed),r.push(...a.missing.map(e=>`${i.id}: ${e}`))}return{settled:t.length===0,pending:t,failed:n,missing:r,frames:this.frameN}}settled(e={}){let t=e.timeout??3e4,n=Date.now();return new Promise((r,i)=>{let a=!0,o=()=>{if(this.disposed){i(new K(`E-WORLD-003`,`the world was disposed while settling`,{fix:`await settled() before dispose()`}));return}let s=this.report({hold:e.hold});if(s.settled&&!a){let t=s.failed.length||e.strict&&s.missing.length;if(t&&e.strict){i(new K(`E-SETTLE-002`,`the view settled with ${s.failed.length} failure(s) and ${s.missing.length} missing: ${[...s.failed.map(e=>`${e.what} (${e.error})`),...s.missing].slice(0,8).join(`; `)}`,{fix:`check the network and the sources; in replay / offline, prepare the view first (loader.ensure)`,details:s}));return}r(!t)}else a=!1,Date.now()-n>t?e.strict?i(new K(`E-SETTLE-001`,`not settled after ${t} ms: ${s.pending.slice(0,8).join(`; `)}${s.pending.length>8?` … (${s.pending.length})`:``}`,{fix:`give it longer (timeout), check the network, or stop the camera`,details:s})):r(!1):this.renderer?.isRunning?(this.once(`frame`,o),this.invalidate()):(e.hold&&(this.holdMs+=1e3/30),this.step(e.hold?0:this.renderer?.clock&&typeof this.renderer.clock==`object`?1e3/this.renderer.clock.fps:16.7),setTimeout(o,e.hold?4:0))};o()})}recordingNow=!1;get recording(){return this.recordingNow}async*record(e={}){let t=this.renderer;if(!t)throw new K(`E-WORLD-003`,`a headless world has no frames to record`,{fix:`give world() a canvas`});let n=e.fps??30,r=e.ahead??2,i=e.timeout??6e4;for(let[e,t,a]of[[`fps`,n,0],[`ahead`,r,-1e-9],[`timeout`,i,0]])if(!(Number.isFinite(t)&&t>a))throw new K(`E-WORLD-006`,`record: ${e} is ${t}`,{where:`record`,fix:`give ${e} as a finite number above ${a>0?a:0}, or leave it out`});if(e.frames!==void 0&&!(Number.isInteger(e.frames)&&e.frames>=0))throw new K(`E-WORLD-006`,`record: frames is ${e.frames}`,{where:`record`,fix:`give frames as a whole number, 0 or more`});if(e.seconds!==void 0&&!(Number.isFinite(e.seconds)&&e.seconds>=0))throw new K(`E-WORLD-006`,`record: seconds is ${e.seconds}`,{where:`record`,fix:`give seconds as a finite number, 0 or more`});if(e.size!==void 0&&!(Array.isArray(e.size)&&e.size.length===2&&e.size.every(e=>Number.isInteger(e)&&e>0&&e<=8192)))throw new K(`E-WORLD-006`,`record: size is ${JSON.stringify(e.size)}`,{where:`record`,fix:`give size as [width, height] in whole pixels (1..8192), or leave it out`});if(this.recordingNow)throw new K(`E-WORLD-006`,`record: a recording is under way`,{where:`record`,fix:`finish it (or break out of its loop) first`});let a=e.frames??(e.seconds===void 0?1/0:Math.round(e.seconds*n)),o=1e3/n,s=this.pathOf(e.path,n),c=t.isRunning;this.stop(),this.recordingNow=!0,this.gov?.reset(),e.size&&t.pin(e.size);try{if(s){let e=Number.isFinite(a)?a/n:r,t=[];for(let n=0;n<=Math.min(r,e)+1e-9;n+=.5){let e=s(n);e&&t.push(e)}let o=[];for(let n=0,r=Math.max(2,e/60);n<=e+1e-9;n+=r){let e=s(n);e&&(o.push(e),t.push({...e,dist:e.dist*4}))}if(t.length){let e=this.prefetch(t,{depth:2,timeout:Math.min(i,12e4)});this.aheadLevels=this.levelsOf(o),await e}this.aheadLevels=this.levelsOf(o)}let t=-1/0;for(let c=0;c<a;c++){c>0&&this.step(o);let l=c/n;if(s&&l-t>=.5-1e-9){t=l;let e=[];for(let t=.5;t<=r+1e-9&&!(Number.isFinite(a)&&l+t>a/n);t+=.5){let n=s(l+t);n&&e.push(n)}if(e.length){let t=this.aheadLevels;this.prefetch(e,{timeout:(r+1)*4e3}),t&&(this.aheadLevels=/* @__PURE__ */ new Set([...t,...this.aheadLevels??[]]))}}let u=this.nowReal(),d=await this.holdStill(i),f=await this.frame({attribution:e.attribution});yield{index:c,t:l,width:f.width,height:f.height,pixels:f.pixels,settled:d.settled,pending:d.pending,waitedMs:Math.round(this.nowReal()-u)}}}finally{this.recordingNow=!1,this.aheadLevels=null,e.size&&t.pin(null),c&&!this.disposed&&this.start()}}pathOf(e,t){if(e===void 0){let e=this.shot;if(!e||!this.camera.driving)return null;let t=e.progress()*(e.seconds??0),n=this.moveWorld(),r=this.timeMs,i=this.clockRate;return a=>e.poseAt(t+a,n,this.camInfo(),r+a*i*1e3)}if(typeof e==`function`)return t=>{let n=e(t);return n?`pitch`in n?n:this.camera.framed(hr(n)):null};if(`aim`in e){let t=new ti(e),n=this.moveWorld(),r=this.timeMs,i=this.clockRate;return e=>t.poseAt(e,n,this.camInfo(),r+e*i*1e3)}let n=e,r=n.list[0].at;return e=>this.camera.framed(n.pose(r+e))}async holdStill(e){let t=Date.now();for(let n=!0;;n=!1){let r=this.report({hold:!0});if(!n&&r.settled&&!this.gates?.swap)return r;if(this.disposed)throw new K(`E-WORLD-003`,`the world was disposed while recording`,{fix:`finish the recording before dispose()`});if(Date.now()-t>e)return{...r,settled:!1};this.holdMs+=1e3/30,this.step(0),await new Promise(e=>setTimeout(e,4))}}async frame(e={}){if(!this.renderer)throw new K(`E-WORLD-003`,`a headless world has no frames`,{fix:`give world() a canvas`});let t=await this.renderer.read(),n=e.attribution===!0?this.attribution().join(` · `):typeof e.attribution==`string`?e.attribution:``;return n?xo(t,n):t}stats(){let e=this.elements.map(e=>e.tileStats?.()).find(Boolean),t=this.renderer;return{api:this.api,reason:this.reason,quality:this.profile?.quality??null,device:this.graphics?.caps.device??``,fps:t?.fps??0,cpuMs:t?.cpuMs??0,frames:this.frameN,draws:this.graphics?.stats.draws??0,passes:this.graphics?.stats.passes??0,uploadBytes:this.graphics?.stats.uploadBytes??0,heapBytes:(typeof performance<`u`?performance.memory?.usedJSHeapSize:0)??0,drawn:t?.drawn??0,skipped:t?.skipped??0,pixelRatio:t?.pixelRatio??0,size:this.graphics?[this.graphics.width,this.graphics.height]:[0,0],gpuBytes:this.graphics?.bytes??0,tiles:e??null,loader:this.loader.stats(),zoom:this.camera.zoom,elements:this.sorted.map(e=>({id:e.id,kind:e.kind,shown:e.shown}))}}telemetry(){let e=this.graphics,t=this.renderer,n=[...this.recentGaps].sort((e,t)=>e-t),r=e=>n.length?+n[Math.min(n.length-1,Math.floor(n.length*e))].toFixed(1):0,i=e?.memory()??[],a=1048576,o=/* @__PURE__ */ new Map,s=this.elements.map(e=>e.id).sort((e,t)=>t.length-e.length);for(let e of i){let t=e.label.startsWith(`solids `)?e.label.slice(7):e.label,n=s.find(e=>t===e||t.startsWith(`${e} `))??`(world)`;o.set(n,(o.get(n)??0)+e.bytes)}let c=e=>{let t=0;for(let n of e)t+=n.count*(n.instanceCount??1)/(n.program?.desc?.topology===`lines`?2:3);return Math.round(t)},l=this.state,u=this.framesOpts,d=this.elements.map(e=>e.tileStats?.()).find(Boolean)??null,f=this.sorted.map(e=>{let t=this.drawsOf.get(e)??[],n=e.shown>0||e.target>0,r=0;if(n)try{r=e.wantsFrames(l),u.rate&&r>0&&(r=u.rate(e,r,l))}catch{r=0}return{id:e.id,kind:e.kind,shown:e.shown,draws:n?t.length:0,triangles:n?c(t):0,ms:+(this.layerMs.get(e.id)??0).toFixed(2),gpuMB:+((o.get(e.id)??0)/a).toFixed(1),wants:Math.min(r,u.max??1/0)}}),p=t?this.wanted(this.nowReal()):{rate:0,who:``};return{frameMs:r(.5),frameP95:r(.95),cpuMs:+(t?.cpuMs??0).toFixed(2),gpuMs:e?.stats.gpuMs??null,draws:e?.stats.draws??0,triangles:c(this.lastDraws),memoryMB:+((e?.bytes??0)/a).toFixed(1),tiles:d,loading:this.report().pending.length,dropped:this.gov?.drops??[],budget:this.gov?{...this.gov.budget}:null,quality:{now:l.render.quality,auto:this.profile?.quality??null,tier:this.profile?.tier??null,why:this.qualityWhy()},frames:{fps:t?.fps??0,wants:p.rate,who:p.who,resting:t?.resting??!1,restedMs:Math.round(t?.rested??0),drawn:t?.drawn??0,every:this.everies.map(e=>({id:e.id,fps:e.fps,ms:+e.ms.toFixed(2),budgetMs:e.budgetMs}))},elements:f,memory:[...o].map(([e,t])=>({id:e,MB:+(t/a).toFixed(1)})).sort((e,t)=>t.MB-e.MB)}}qualityWhy(){return this.qualityDrop?`the governor dropped it (frames over the budget)`:this.qualityNow===null?this.qualityFn?`your function`:this.profile?.why===`as asked`?`set (the quality option)`:this.profile?.why?`auto: ${this.profile.why}`:`auto: this device`:`set (w.setQuality)`}recentGaps=[];layerMs=/* @__PURE__ */ new Map;pickMs=0;longTasks=[];longObs=null;frameProfile(){if(!this.longObs&&!this.disposed&&typeof PerformanceObserver<`u`&&PerformanceObserver.supportedEntryTypes?.includes(`longtask`))try{this.longObs=new PerformanceObserver(e=>{for(let t of e.getEntries())this.longTasks.push({at:t.startTime,ms:t.duration});this.longTasks.length>200&&this.longTasks.splice(0,100)}),this.longObs.observe({type:`longtask`,buffered:!0})}catch{}let e=this.renderer,t=this.graphics,n=this.nowReal(),r=this.longTasks.filter(e=>n-e.at<1e3),i=this.elements.map(e=>e.frameCost?.()).find(Boolean)??null;return{tickMs:e?.tickMs??0,submitMs:e?.submitMs??0,elements:[...this.layerMs].sort((e,t)=>t[1]-e[1]).slice(0,6).map(([e,t])=>[e,+t.toFixed(2)]),surface:i,pickMs:+this.pickMs.toFixed(2),uploads:t?.stats.uploads??0,uploadBytes:t?.stats.uploadBytes??0,draws:t?.stats.draws??0,size:t?[t.width,t.height]:[0,0],pixelRatio:e?.pixelRatio??0,longTasks:r.length,longTaskMs:Math.round(r.reduce((e,t)=>e+t.ms,0)),gpuMs:t?.stats.gpuMs??null,uploadMaxMs:t?.stats.uploadMaxMs??0,passes:t?.stats.passes??0}}attribution(){return[...new Set(this.sorted.filter(e=>e.shown>0||e.kind===`basemap`).flatMap(e=>e.attribution()).filter(Boolean))]}on(e,t){let n=this.ev,r=n[e]??=/* @__PURE__ */ new Set;return r.add(t),()=>r.delete(t)}once(e,t){let n=this.on(e,(...e)=>{n(),t(...e)})}emit(e,...t){let n=this.ev[e];if(n)for(let r of[...n])try{r(...t)}catch(t){e!==`error`&&this.error(t)}}error(e){this.remote?.send(`error`,{msg:String(e?.message??e).slice(0,500),code:e?.code},`warn`),this.ev.error?.size?this.emit(`error`,e):this.errorHandler(e)}async pick(e,t,n={}){let r=this.renderer,i=this.graphics;if(!r||!i||this.disposed)return null;let a=r.pixelRatio,o=Math.max(0,Math.round((n.radius??4)*a)),s=2*o+1;(!this.pickTarget||this.pickTarget.width!==s)&&(this.pickTarget?.destroy(),this.pickTarget=i.target(s,s,{label:`picking`}));let c=n.elements?/* @__PURE__ */ new Set([...n.elements,...this.elements.filter(e=>e.owner&&n.elements.includes(e.owner.id)).map(e=>e.id)]):null,l=[];for(let e of this.sorted)e.groundPick&&(this.drawsOf.get(e)??[]).some(e=>e.ground)&&(!c||c.has(e.id))&&l.push(e);let u=/* @__PURE__ */ new Set;for(let e of l)for(let t of this.drawsOf.get(e)??[])t.ground&&u.add(t);let d=()=>{let t=/* @__PURE__ */ new Map;for(let e of this.sorted)for(let n of this.drawsOf.get(e)??[])t.set(n,e);let n=this.compareView,i=n?e<n.at*(this.renderer?.cssWidth??1)?n.leftHides:n.rightHides:null;return this.lastDraws.filter(e=>e.pickProgram&&!u.has(e)&&!(i&&t.get(e)&&i.has(t.get(e)))&&(!c||!t.get(e)||t.get(e).kind===`basemap`||c.has(t.get(e).id))).map(e=>({...e,program:e.pickProgram(),blocks:e.blocks[0]===this.subjectBlock?[r.frameBlock,...e.blocks.slice(1)]:e.blocks}))},f=()=>{if(!l.length)return null;let n=this.groundUnder(e,t);if(!n)return null;for(let r=l.length-1;r>=0;r--){let i=l[r],a=i.groundPick(n[0],n[1]);if(a===null)continue;let o=i.pickInfo(a);if(!o)continue;let s=this.heightAt(n[0],n[1]);return{...o,at:[n[0],n[1],Number.isNaN(s)?0:s],x:e,y:t}}return null},p=/* @__PURE__ */ new Set;for(let e of this.sorted)if(e.kind===`basemap`||e.kind===`sky`)for(let t of this.drawsOf.get(e)??[])p.add(t);if(!this.lastDraws.some(e=>e.pickProgram&&!u.has(e)&&!p.has(e)))return f();let m=d();if(!m.length)return f();this.picking=!0;let h;try{if(m.some(e=>!e.program.ok)&&(await Promise.all(m.map(e=>e.program.ready.catch(()=>{}))),m=d()),this.disposed||!this.pickTarget)return this.picking=!1,null;let n=i.width,o=i.height,c=e*a/n*2-1,l=1-t*a/o*2,u=n/s,f=o/s,p=this.pickFrame;p.set(r.frameData),p[36]=1,p.set([u,f,-c*u,-l*f],40),p[60]=s,p[61]=s,this.lastPick=this.nowReal(),r.frameBlock.write(p);let g=i.frame(m,{clear:[0,0,0,0],target:this.pickTarget,read:!0});r.frameBlock.write(r.frameData),h=await g}catch(e){return this.picking=!1,this.error(e),null}this.picking=!1;let g=-1,_=1/0;for(let e=0;e<s;e++)for(let t=0;t<s;t++){let n=(e*s+t)*4;if(!h[n])continue;let r=(t-o)**2+(e-o)**2;r<_&&(_=r,g=n)}if(g<0)return f();let v=h[g],y=h[g+1]+h[g+2]*256+h[g+3]*65536,b=this.elements.find(e=>e.pickNo===v);if(!b)return null;let x=b.pickInfo(y);if(!x)return null;let S=this.camera.lonLatAt(e,t),C=S?this.heightAt(S[0],S[1]):NaN;return{...x,at:S?[S[0],S[1],Number.isNaN(C)?0:C]:null,x:e,y:t}}groundUnder(e,t){let n=this.camera,r=this.exaggeration,i=n.groundY,o=null;for(let s=0;s<6;s++){let s=n.groundHit(e,t,i);if(!s)return o;o=a(n,s[0],s[1],s[2]);let c=this.heightAt(o[0],o[1]);if(Number.isNaN(c))break;let l=c*r;if(Math.abs(l-i)<.5)break;i=l}return o}drawsOf=/* @__PURE__ */ new Map;pickFrame=/* @__PURE__ */ new Float32Array(112);lastPick=-1e9;pickedAt=null;pickSoon(){if(!this.hoverWanted||this.picking||!this.pointer||this.nowReal()-this.lastPick<=50)return;let e=this.pointer,t=this.pickedAt;this.hoverWanted=!1,!(t&&Math.abs(t.x-e.x)<1&&Math.abs(t.y-e.y)<1&&!this.dirtySincePick)&&(this.pickedAt={x:e.x,y:e.y},this.dirtySincePick=!1,queueMicrotask(()=>{this.hoverAt(e).catch(()=>{this.picking=!1})}))}dirtySincePick=!0;nowReal(){return typeof performance<`u`?performance.now():Date.now()}async hoverAt(e){let t=this.nowReal(),n=await this.pick(e.x,e.y,{radius:3});this.pickMs=this.nowReal()-t;let r=this.camera.lonLatAt(e.x,e.y);await this.setHover(n,{x:e.x,y:e.y,lon:r?.[0]??null,lat:r?.[1]??null},e.cx,e.cy)}async setHover(e,t,n=0,r=0){let i=this.hoverNow;if(!(!i&&!e||i&&e&&i.element===e.element&&String(i.id)===String(e.id))){if(this.invalidate(),i)try{this.element(i.element).hovered(null)}catch{}if(e){let t=this.element(e.element);t.hovered(t.groupOf?t.groupOf(String(e.id)):/* @__PURE__ */ new Set([String(e.id)]))}this.hoverNow=e,this.user.hover=e?String(e.id):null,this.stateCache=null;let n=e?this.element(e.element).props:null;if(i&&(!e||i.element!==e.element))try{this.element(i.element).props.onHover?.(null,this.state)}catch{}n?.onHover?.(e,this.state),this.formFor(`hover`,e,e?n?.hover:void 0),e&&this.remember(e),this.canvasEl&&(this.canvasEl.style.cursor=e?n?.hover&&n.hover.cursor||(n?.onTap||n?.select?`pointer`:``):``),this.emit(`hover`,e,t)}this.tooltip(e,n,r)}tooltip(e,t,n){let r=e?this.element(e.element).props.tooltip:void 0,i=e&&r?r(e.d,this.state)??null:null;if(this.tipOpts?.show){this.tipOpts.show(i,t,n);return}if(typeof document>`u`)return;if(!i){this.tip&&(this.tip.style.display=`none`);return}if(!this.tip){let e=document.createElement(`div`);e.className=this.tipOpts?.className??`aslnut-tooltip`,this.tipOpts?.className||Object.assign(e.style,{position:`fixed`,zIndex:`20`,pointerEvents:`none`,background:`rgba(12,16,24,.92)`,color:`#eef2f6`,font:`12px/1.35 system-ui, sans-serif`,padding:`5px 8px`,borderRadius:`6px`,border:`1px solid rgba(255,255,255,.14)`,maxWidth:`260px`,whiteSpace:`pre-line`,boxShadow:`0 4px 14px rgba(0,0,0,.35)`}),document.body.appendChild(e),this.tip=e}let a=this.tip;typeof i==`string`?a.textContent=i:(a.textContent=``,a.appendChild(i)),a.style.display=`block`;let o=a.offsetWidth,s=a.offsetHeight,c=innerWidth;a.style.left=`${Math.max(4,Math.min(c-o-4,t+14))}px`,a.style.top=`${Math.max(4,n-s-12<4?n+18:n-s-12)}px`}async tapAt(e,t,n){let r=await this.pick(e,t,{radius:n?12:4}),i=this.camera.lonLatAt(e,t),a={x:e,y:t,lon:i?.[0]??null,lat:i?.[1]??null};if(n){let n=this.canvasEl?.getBoundingClientRect();await this.setHover(r,a,e+(n?.left??0),t+(n?.top??0))}if(r){let e=this.element(r.element).props;if(this.remember(r),e.select===`tap`)this.select([r.id]);else if(e.select===`toggle`){let e=String(r.id),t=this.user.selection;this.select(t.includes(e)?t.filter(t=>t!==e):[...t,e])}e.onTap?.(r,this.state)}this.emit(`tap`,r,a)}selectedBy(e){return this.pickedById.get(e)?.element}select(e){if(this.invalidate(),this.user.selection=(e??[]).map(String),this.stateCache=null,this.formLayer||this.sorted.some(e=>e.props.selected&&e.props.selected.shape)){let e=new Set(this.user.selection.map(e=>`sel:${e}`));for(let t of this.formLayer?.keysShown()??[])t.startsWith(`sel:`)&&!e.has(t)&&this.formFor(t,null,void 0);for(let e of this.user.selection){let t=this.pickedById.get(e);if(t)try{this.formFor(`sel:${e}`,t,this.element(t.element).props.selected)}catch{}}}return this.emit(`select`,this.user.selection),this}toggle(e,t){return this.invalidate(),this.user.toggles={...this.user.toggles,[e]:t??!this.user.toggles[e]},this.stateCache=null,this}reveal(e,t={}){let n=this.element(e);return n.forced=!0,t.seconds!==void 0&&(n.once=t.seconds),this.ownedOnce(n,t.seconds),this.invalidate(),this.emit(`elements`,{what:`shown`,ids:[e]}),this}hide(e,t={}){let n=this.element(e);return n.forced=!1,t.seconds!==void 0&&(n.once=t.seconds),this.ownedOnce(n,t.seconds),this.invalidate(),this.emit(`elements`,{what:`shown`,ids:[e]}),this}setTime(e,t={}){return this.invalidate(),this.timeRest=0,e===`now`?(this.timeIsNow=!0,this.time=/* @__PURE__ */ new Date,this.clockRate=1):(this.timeIsNow=!1,this.time=new Date(e.getTime()),this.clockRate=t.rate??0),this.stateCache=null,this}get timeRate(){return this.clockRate}setTimeRate(e){return this.timeIsNow&&e===1||(this.timeIsNow=!1,this.clockRate=e,this.invalidate(),this.stateCache=null),this}setTimeZone(e){return this.zones=this.zoneTracker(e),this.stateCache=null,this.invalidate(),this}get timeZone(){return this.zones.zoneAt(this.camera.lon,this.camera.lat)}zoneTracker(e){return new $n(e,()=>{this.stateCache=null,this.invalidate()})}setQuality(e){return this.invalidate(),this.qualityNow=e,this.stateCache=null,this}capture(e){return ya(this,e)}apply(e,t){return ba(this,e,t)}release(e,t={}){let n=this.element(e);return n.forced=null,t.seconds!==void 0&&(n.once=t.seconds),this.ownedOnce(n,t.seconds),this.invalidate(),this.emit(`elements`,{what:`shown`,ids:[e]}),this}alive(){if(this.disposed)throw new K(`E-WORLD-003`,`the world was disposed`,{fix:`make a new world`})}get isDisposed(){return this.disposed}dispose(){if(!this.disposed){this.disposed=!0,this.renderer?.stop(),this.heartbeat&&clearInterval(this.heartbeat),this.heartbeat=null,this.remote?.dispose(),this.remote=null,this.gov=null,this.longObs?.disconnect(),this.longObs=null,this.longTasks=[],this.detachControls?.(),this.detachControls=null,this.tip?.remove(),this.tip=null,this.pickTarget=null,this.creditsEl?.remove(),this.creditsEl=null,this.shadowTarget=null,this.shadowBuf=null,this.noShadow=null,this.subjectBlock=null,this.subjectDraws.clear(),this.camera.stop();for(let e of this.elements)try{e.detach()}catch(e){this.errorHandler(e)}this.elements=[],this.sorted=[],this.renderer?.dispose(),this.ownLoader&&this.loader.dispose(),this.ev={},this.paramDefs.clear(),this.readDefs.clear(),this.pointDefs.clear()}}};async function yo(e,t={}){let n=t.body??null,r=!(t.loader instanceof Li),i=t.memory,a=i?{...i.limitMB===void 0?{}:{memory:i.limitMB*1048576},...i.evict===void 0?{}:{evict:i.evict}}:{},o=t.loader instanceof Li?t.loader:new Li({...t.loader,...a});i&&t.loader instanceof Li&&t.loader.setMemory(i.limitMB===void 0?t.loader.budget:i.limitMB*1048576,i.evict);let s=new wr;n?.flat&&(s.flat=!0,s.minDist=1),t.frame&&(s.frame=t.frame);let c=null,l=`headless (no canvas)`,u=null;if(e){let n=await oa(e,t.graphics??`auto`);c=n.graphics,l=n.reason;let r=t.render?.quality??t.quality,i=typeof r==`string`&&r!==`auto`?r:void 0;u=ca(c.caps,{quality:i}),i===void 0&&u.why&&(l+=` · quality ${u.quality} (${u.why})`)}let d=new vo({body:n,camera:s,loader:o,ownLoader:r,graphics:c,reason:l,profile:u,canvas:e,opts:t});if(e&&`getBoundingClientRect`in e){let t=e.getBoundingClientRect();s.width=Math.max(1,t.width),s.height=Math.max(1,t.height)}let f=t.view??n?.view??{lon:0,lat:0,zoom:1.5},p=typeof f==`string`?d.views.get(f):f;if(p){pr(p,`view`);let e=s.framed(hr(p)).dist;e<s.minDist&&(s.minDist=e)}return d.go(f),t.observation&&d.observe(t.observation),n&&d.add(...n.elements()),c&&t.autostart!==!1&&d.start(),d}const bo=e=>e instanceof Ai;function xo(e,t){if(typeof OffscreenCanvas>`u`)return e;let n=new OffscreenCanvas(e.width,e.height).getContext(`2d`);if(!n)return e;n.putImageData(new ImageData(new Uint8ClampedArray(e.pixels),e.width,e.height),0,0);let r=Math.max(10,Math.round(e.height*.016)),i=Math.round(r*.5);n.font=`500 ${r}px system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`;let a=t;for(;a.length>8&&n.measureText(a).width>e.width-4*i;)a=`${a.slice(0,-2)}`;a!==t&&(a=`${a.slice(0,-1)}…`);let o=n.measureText(a).width;n.fillStyle=`rgba(0,0,0,0.55)`,n.fillRect(e.width-o-2*i,e.height-r-2*i,o+2*i,r+2*i),n.fillStyle=`rgba(255,255,255,0.92)`,n.textBaseline=`bottom`,n.fillText(a,e.width-o-i,e.height-i);let s=n.getImageData(0,0,e.width,e.height);return{width:e.width,height:e.height,pixels:new Uint8Array(s.data.buffer)}}const So=e=>{if(!e||typeof e!=`object`)return!1;let t=Object.getPrototypeOf(e);return t===Object.prototype||t===null};function Co(e,t,n=3){if(Object.is(e,t))return!0;if(n<=0||!e||!t||typeof e!=`object`||typeof t!=`object`)return!1;if(e instanceof Date&&t instanceof Date)return e.getTime()===t.getTime();if(Array.isArray(e)&&Array.isArray(t)){if(e.length!==t.length)return!1;for(let r=0;r<e.length;r++)if(!Co(e[r],t[r],n-1))return!1;return!0}if(!So(e)||!So(t))return!1;let r=Object.keys(e);if(r.length!==Object.keys(t).length)return!1;for(let i of r)if(!(i in t)||!Co(e[i],t[i],n-1))return!1;return!0}function wo(e,t){let n=null;for(let r of/* @__PURE__ */ new Set([...Object.keys(e),...Object.keys(t)]))r!==`id`&&!Co(e[r],t[r])&&((n??={})[r]=t[r]);return n}const To=[{key:e=>e.views,set:(e,t)=>{for(let[n,r]of Object.entries(t.views??{}))e.addView(n,r)}},{key:e=>e.time,set:(e,t)=>{e.setTime(t.time??`now`)}},{key:e=>e.timeZone,set:(e,t)=>{e.setTimeZone(t.timeZone)}},{key:e=>e.render?.quality??e.quality,set:(e,t)=>{let n=t.render?.quality??t.quality;e.setQuality(n===`lite`||n===`realistic`?n:null)}},{key:e=>e.render?.shadows,set:(e,t)=>{e.setShadows(t.render?.shadows??!1)}},{key:e=>e.theme?[e.theme.name,e.theme.spec]:e.theme,set:(e,t)=>{e.setTheme(t.theme??null)}},{key:e=>e.budget,set:(e,t)=>{let n=t.budget;e.setBudget(n===`auto`||n===void 0?e.profile?.budget??null:n===!1?null:n)}},{key:e=>e.priorities,set:(e,t)=>{e.setPriorities(t.priorities??{})}},{key:e=>e.observation,set:(e,t)=>{e.observe(t.observation??null)}}];function Eo(e,t={}){let n=/* @__PURE__ */ new Map,r=t.options??{},i=[],a=!1,o=t=>{e.isDisposed||e.error(t)},s=e=>{try{e()}catch(e){o(e)}},c=t=>e.list.includes(t);function l(t){let n=t.filter(c);for(;n.length;){let t=n.filter(e=>n.some(t=>t!==e&&(t.above===e.id||t.below===e.id))),r=t.length<n.length?n.filter(e=>!t.includes(e)):n;for(let t of r)s(()=>{e.remove(t.id)});n=r===n?[]:t}}function u(t){if(e.isDisposed)return;let r=/* @__PURE__ */ new Map,u=[],d=[],f=[];for(let e of t){if(!e||typeof e.frame!=`function`){o(new K(`E-WORLD-001`,`elements take aslnut elements`,{fix:`elements={[layer({ id: 'roads', data, line: { color } })]}`}));continue}if(r.has(e.id)){o(new K(`E-WORLD-001`,`two elements with id '${e.id}'`,{where:e.id,fix:`give each element its own id`}));continue}r.set(e.id,e);let t=n.get(e.id);if(!t){u.push(e);continue}let i=e.props;if(t.el===e){t.props=i;continue}if(e.constructor!==t.el.constructor||e.kind!==t.el.kind){d.push(t.el),n.delete(e.id),u.push(e);continue}let a=wo(t.props,i);t.props=i,a&&f.push([t.el,a])}if(!a&&n.size){for(let e of u)if(e.props.id===void 0){a=!0,o(new K(`E-WORLD-001`,`element '${e.id}' has no id of its own: a new one is added each time it is made`,{where:e.id,fix:`give it an id: layer({ id: 'roads', … })`}));break}}let p=[];for(let[e,t]of n)r.has(e)||(p.push(t.el),n.delete(e));l(d);let m=u.filter(t=>e.list.some(e=>e.id===t.id));for(let e of m)o(new K(`E-WORLD-001`,`two elements with id '${e.id}'`,{where:e.id,fix:`the world has one already (its body's, or one added by hand): give this one its own id`}));let h=u.filter(e=>!m.includes(e));h.length&&s(()=>{e.add(...h)});for(let e of h)c(e)&&n.set(e.id,{el:e,props:e.props});for(let[e,t]of f)s(()=>{e.setProps(t)});l(p),i=[...r.keys()].filter(e=>n.has(e))}function d(n,i=t.viewChange){if(e.isDisposed)return;let a=r;r=n;for(let t of To)Co(t.key(a),t.key(n))||s(()=>{t.set(e,n)});if(i!==void 0&&n.view!==void 0&&!Co(a.view,n.view)){let t=n.view;s(()=>{i===`go`?e.go(t):e.fly(t,i===`fly`?void 0:i).catch(o)})}}return{elements:u,options:d,get ids(){return i}}}const Do=[[0,-1],[0,1],[1,-1],[1,1]];function Oo(e,t,n,r,i,a){let o=t.line,s=r.P,c=o.length;if(t.closed&&c>1&&o[0][0]===o[c-1][0]&&o[0][1]===o[c-1][1]&&c--,c<2)return r.n[a]=0,0;r.n[a]=c;for(let t=0;t<c;t++){let n=o[t],r=2*(i+t),a=e.x(n[0]);t&&(a=Un(a,s[r-2])),s[r]=a,s[r+1]=e.y(n[1])}let l=t.closed?c:c-1,u=t.cuts,d=1,f=0,p=0;for(let t=0;t<l;t++){let a=2*(i+t),o=2*(i+(t+1===c?0:t+1)),l=Se(e,s[a],s[a+1],s[o],s[o+1]),m=Math.max(1,Math.min(256,Math.ceil(l/n)));r.Lm[i+t]=l,r.K[i+t]=m,d+=m,f+=l,u&&u[t]||(p+=m)}return r.len[a]=f,r.pts+=d,r.segs+=p,c}function ko(e,t,n,r,i,a){let o=a.o,s=a.xy,c=a.ll,l=a.fr,u=r.P,d=r.Lm,f=r.K,p=e.line,m=e.along,h=e.reverse,g=e.closed?t:t-1,_=a.q,v=_;s[2*v]=u[2*i],s[2*v+1]=u[2*i+1],c[2*v]=p[0][0],c[2*v+1]=p[0][1];{let e=m?m(0):0;l[v]=h?1-e:e}v++;let y=0;for(let e=0;e<g;e++){let r=e+1===t?0:e+1,a=2*(i+e),g=2*(i+r),_=f[i+e],b=d[i+e],x=u[a],S=u[a+1],C=u[g],w=u[g+1];for(let t=1;t<=_;t++){let i=t/_,a=x+(C-x)*i,u=S+(w-S)*i;if(s[2*v]=a,s[2*v+1]=u,t<_)c[2*v]=o.lonOf(a),c[2*v+1]=o.latOf(u);else{let e=p[r];c[2*v]=e[0],c[2*v+1]=e[1]}let d=m?m(e+i):n>0?(y+b*i)/n:0;l[v]=h?1-d:d,v++}y+=b}a.q=v;let b=v-_,x=e.cuts,S=e.closed,C=e.f,w=a.records,T=a.seg,E=a.lens,D=a.first,O=a.pieces,k=a.sq;for(let e=0,t=0;e<g;e++){let r=f[i+e];if(!(x&&x[e]))for(let e=0;e<r;e++){let r=t+e,i=_+r,a=r>0?i-1:S?_+b-2:-1,o=r+2<b?i+2:S?_+1:-1;if(w){let e=k*5;T[e]=i,T[e+1]=i+1,T[e+2]=a,T[e+3]=o,T[e+4]=C,E[k]=n}else{D[k]=i;let e=k*20;O[e]=s[2*i],O[e+1]=s[2*i+1],O[e+3]=l[i],O[e+4]=s[2*i+2],O[e+5]=s[2*i+3],O[e+7]=l[i+1],a>=0&&(O[e+8]=s[2*a],O[e+9]=s[2*a+1],O[e+11]=1),o>=0&&(O[e+12]=s[2*o],O[e+13]=s[2*o+1],O[e+15]=1),O[e+16]=C,O[e+17]=n}k++}t+=r}a.sq=k}var Ao=class{r;w=null;l=0;at=0;done=!1;segs;o;lines;step;opts;constructor(e,t,n,r,i={}){this.segs=e,this.o=t,this.lines=n,this.step=r,this.opts=i;let a=0;for(let e of n)a+=e.line.length;this.r={P:new Float64Array(a*2),Lm:new Float64Array(a),K:new Int32Array(a),n:new Int32Array(n.length),len:new Float64Array(n.length),pts:0,segs:0}}run(e=1/0){if(this.done)return!0;let t=e===1/0?0:performance.now(),n=this.lines,r=this.r,i=n.length,a=()=>e!==1/0&&performance.now()-t>e;if(!this.w){for(;this.l<i;)if(this.at+=Oo(this.o,n[this.l],this.step,r,this.at,this.l),this.l++,!(this.l&127)&&a())return!1;let e=(this.opts.form??`records`)===`records`,t=r.pts,o=r.segs,s=Math.max(4,this.opts.row??4);if(this.w={o:this.o,records:e,q:0,sq:0,ll:new Float64Array(t*2),xy:new Float32Array(t*2),fr:new Float32Array(t),seg:e?new Int32Array(o*5):/* @__PURE__ */ new Int32Array,lens:e?new Float32Array(o):/* @__PURE__ */ new Float32Array,first:e?/* @__PURE__ */ new Int32Array:new Int32Array(o),pieces:e?/* @__PURE__ */ new Float32Array:new Float32Array(Math.max(1,Math.ceil(o*20/s))*s)},this.l=0,this.at=0,a())return!1}let o=this.w;for(;this.l<i;){let e=r.n[this.l];if(e&&(ko(n[this.l],e,r.len[this.l],r,this.at,o),this.at+=e),this.l++,!(this.l&127)&&a())return!1}return this.segs.finish(o,r.pts),this.done=!0,!0}},jo=class{lonlat=/* @__PURE__ */ new Float64Array;xy=/* @__PURE__ */ new Float32Array;frac=/* @__PURE__ */ new Float32Array;h=/* @__PURE__ */ new Float32Array;seg=/* @__PURE__ */ new Int32Array;len=/* @__PURE__ */ new Float32Array;data=/* @__PURE__ */ new Float32Array;index=/* @__PURE__ */ new Uint32Array;first=/* @__PURE__ */ new Int32Array;pieces=/* @__PURE__ */ new Float32Array;form=`records`;count=0;build(e,t,n,r={}){return new Ao(this,e,t,n,r).run(),this}finish(e,t){let n=e.records;if(this.form=n?`records`:`pieces`,this.count=e.sq,this.lonlat=e.ll,this.seg=e.seg,this.len=e.lens,this.first=e.first,this.pieces=e.pieces,n){this.xy=e.xy,this.frac=e.fr,this.h=new Float32Array(t),this.data=new Float32Array(this.count*20*4),this.index=new Uint32Array(this.count*6);for(let e=0;e<this.count;e++){let t=e*4,n=e*6,r=this.index;r[n]=t,r[n+1]=t+1,r[n+2]=t+2,r[n+3]=t+2,r[n+4]=t+1,r[n+5]=t+3}this.fill()}else this.xy=/* @__PURE__ */ new Float32Array,this.frac=/* @__PURE__ */ new Float32Array,this.h=/* @__PURE__ */ new Float32Array,this.data=/* @__PURE__ */ new Float32Array,this.index=/* @__PURE__ */ new Uint32Array}box(e,t){let n=this.form===`records`?this.seg[e*5]:this.first[e],r=this.lonlat,i=r[2*n],a=r[2*n+1],o=r[2*n+2],s=r[2*n+3];t[0]=Math.min(i,o),t[1]=Math.min(a,s),t[2]=Math.max(i,o),t[3]=Math.max(a,s)}ground(e){for(let t=0;t<this.h.length;t++)this.h[t]=e(this.lonlat[2*t],this.lonlat[2*t+1]);this.fill()}heights(e){for(let t=0;t<this.h.length;t++)this.h[t]=e(t);this.fill()}fill(){if(this.form!==`records`)return;let e=this.data,t=this.seg,n=this.xy,r=this.h,i=this.frac,a=(t,a,o)=>{a<0?(e[t]=0,e[t+1]=0,e[t+2]=0,e[t+3]=0):(e[t]=n[2*a],e[t+1]=n[2*a+1],e[t+2]=r[a],e[t+3]=o<0?i[a]:o)};for(let n=0;n<this.count;n++){let r=n*20*4,i=n*5;a(r,t[i],-1),a(r+4,t[i+1],-1),a(r+8,t[i+2],1),a(r+12,t[i+3],1),e[r+16]=t[i+4],e[r+17]=this.len[n];for(let t=0;t<4;t++){let n=r+t*20;t&&e.copyWithin(n,r,r+18),e[n+18]=Do[t][0],e[n+19]=Do[t][1]}}}};const Mo=`${Be}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,No={vertex:`${Mo}
layout(location = 0) in vec4 a;
layout(location = 1) in vec4 b;
layout(location = 2) in vec4 p0;
layout(location = 3) in vec4 p3;
layout(location = 4) in vec4 x;
out vec4 v_col;
out float v_across;
out float v_halfw;
out float v_along;
out float v_alongPx;
flat out float v_idx;
flat out vec4 v_state;
out vec3 v_world;
flat out float v_value;
vec2 screenOf(vec4 c) {
  return c.xy / c.w * frame.size.xy * 0.5;
}
vec2 perp(vec2 d) {
  return vec2(-d.y, d.x);
}
vec2 dirOf(vec2 a, vec2 b) {
  vec2 d = b - a;
  float l = length(d);
  if (l < 0.0001) {
    return vec2(1.0, 0.0);
  }
  return d / l;
}
vec4 clipAt(vec2 rel, float h) {
  return frame.viewProj * vec4(liftToEye(placeAt(rel, h), u.b.z), 1.0);
}
void main() {
  float end = x.z;
  float side = x.w;
  float idx = x.x;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  float lift = u.origin.w + raiseOf(state) + motion.z;
  float ex = u.origin.z;
  vec3 PA = placeAt(a.xy, a.z * ex + lift);
  vec3 PB = placeAt(b.xy, b.z * ex + lift);
  vec4 cA = frame.viewProj * vec4(liftToEye(PA, u.b.z), 1.0);
  vec4 cB = frame.viewProj * vec4(liftToEye(PB, u.b.z), 1.0);
  if (cA.w <= 0.0 || cB.w <= 0.0) {
    gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
    return;
  }
  vec2 sA = screenOf(cA);
  vec2 sB = screenOf(cB);
  vec2 n = perp(dirOf(sA, sB));
  vec2 m = n;
  if (end < 0.5 && p0.w > 0.5) {
    vec4 c0 = clipAt(p0.xy, p0.z * ex + lift);
    if (c0.w > 0.0) {
      m = normalize(perp(dirOf(screenOf(c0), sA)) + n);
    }
  }
  if (end > 0.5 && p3.w > 0.5) {
    vec4 c3 = clipAt(p3.xy, p3.z * ex + lift);
    if (c3.w > 0.0) {
      m = normalize(n + perp(dirOf(sB, screenOf(c3))));
    }
  }
  float ml = 1.0 / max(dot(m, n), 0.5);
  float w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  w = sizePx(w, u.size, end > 0.5 ? PB : PA);
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  float halfw = w * 0.5 + 1.0;
  vec4 c = end > 0.5 ? cB : cA;
  vec2 off = m * side * halfw * ml;
  gl_Position = clipOut(vec4(c.xy + off / (frame.size.xy * 0.5) * c.w, c.z, c.w));
  vec4 col = vec4(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  vec3 P = end > 0.5 ? PB : PA;
  v_col = col;
  v_across = side * halfw * ml;
  v_halfw = halfw;
  v_along = mix(a.w, b.w, end);
  v_alongPx = v_along * x.y / max(mppAt(P) / max(frame.size.z, 0.001), 0.000001);
  v_idx = idx;
  v_state = state;
  v_world = P;
  v_value = featAt(idx, 3).z;
}
`,fragment:`${Mo}
in vec4 v_col;
in float v_across;
in float v_halfw;
in float v_along;
in float v_alongPx;
flat in float v_idx;
flat in vec4 v_state;
in vec3 v_world;
flat in float v_value;
out vec4 outColor;
void main() {
  vec4 color = v_col;
  float alpha = color.a * clamp(v_halfw - abs(v_across), 0.0, 1.0) * clamp((v_halfw - 1.0) * 2.0, 0.0, 1.0);
  float along = v_along;
  float time = frame.eye.w;
  float value = v_value;
  vec3 position = v_world;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  // dashes, flowing along at u.b.y px a second (0: still)
  float dashP = max(u.a.z + u.a.w, 0.0001);
  float dashAt = v_alongPx - time * u.b.y;
  if (u.a.z > 0.0 && mod(dashAt, dashP) > u.a.z) {
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
`},Po=`${A}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct I { @location(0) a: vec4f, @location(1) b: vec4f, @location(2) p0: vec4f, @location(3) p3: vec4f, @location(4) x: vec4f }
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec4f,
  @location(1) across: f32,
  @location(2) halfw: f32,
  @location(3) along: f32,
  @location(4) alongPx: f32,
  @location(5) @interpolate(flat) idx: f32,
  @location(6) @interpolate(flat) state: vec4f,
  @location(7) world: vec3f,
  @location(8) @interpolate(flat) value: f32,
}
/*@functions*/
fn screenOf(c: vec4f) -> vec2f {
  return c.xy / c.w * frame.size.xy * 0.5;
}
fn perp(d: vec2f) -> vec2f {
  return vec2f(-d.y, d.x);
}
fn dirOf(a: vec2f, b: vec2f) -> vec2f {
  let d = b - a;
  let l = length(d);
  if (l < 0.0001) {
    return vec2f(1.0, 0.0);
  }
  return d / l;
}
fn clipAt(rel: vec2f, h: f32) -> vec4f {
  return frame.viewProj * vec4f(liftToEye(placeAt(rel, h), u.b.z), 1.0);
}
@vertex fn vs(i: I) -> O {
  let end = i.x.z;
  let side = i.x.w;
  let idx = i.x.x;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  let lift = u.origin.w + raiseOf(state) + motion.z;
  let ex = u.origin.z;
  let PA = placeAt(i.a.xy, i.a.z * ex + lift);
  let PB = placeAt(i.b.xy, i.b.z * ex + lift);
  let cA = frame.viewProj * vec4f(liftToEye(PA, u.b.z), 1.0);
  let cB = frame.viewProj * vec4f(liftToEye(PB, u.b.z), 1.0);
  var o: O;
  if (cA.w <= 0.0 || cB.w <= 0.0) {
    o.pos = vec4f(0.0, 0.0, 2.0, 1.0);
    return o;
  }
  let sA = screenOf(cA);
  let sB = screenOf(cB);
  let n = perp(dirOf(sA, sB));
  var m = n;
  // the join: only the neighbour on this vertex's end is needed
  if (end < 0.5 && i.p0.w > 0.5) {
    let c0 = clipAt(i.p0.xy, i.p0.z * ex + lift);
    if (c0.w > 0.0) {
      m = normalize(perp(dirOf(screenOf(c0), sA)) + n);
    }
  }
  if (end > 0.5 && i.p3.w > 0.5) {
    let c3 = clipAt(i.p3.xy, i.p3.z * ex + lift);
    if (c3.w > 0.0) {
      m = normalize(n + perp(dirOf(sB, screenOf(c3))));
    }
  }
  let ml = 1.0 / max(dot(m, n), 0.5);
  var w = u.a.x;
  if (u.a.y > 0.5) {
    w = w * featAt(idx, 3).x;
  }
  // CSS px, or metres where this end is seen, within the layer's range (core › render › drawing › size)
  w = sizePx(w, u.size, select(PA, PB, end > 0.5));
  w = w + u.hover2.y * state.x + u.sel2.y * state.y;
  w = w * frame.size.z * motion.w;
  let halfw = w * 0.5 + 1.0;
  let c = select(cA, cB, end > 0.5);
  let off = m * side * halfw * ml;
  o.pos = clipOut(vec4f(c.xy + off / (frame.size.xy * 0.5) * c.w, c.z, c.w));
  var col = vec4f(mix(u.c.rgb, u.hover.rgb, state.x), mix(u.c.a, 1.0, max(state.x, state.y)));
  if (u.b.w < 0.5) {
    col = colourOf(idx, state);
  }
  let P = select(PA, PB, end > 0.5);
  o.col = col;
  o.across = side * halfw * ml;
  o.halfw = halfw;
  o.along = mix(i.a.w, i.b.w, end);
  o.alongPx = o.along * i.x.y / max(mppAt(P) / max(frame.size.z, 0.001), 0.000001);
  o.idx = idx;
  o.state = state;
  o.world = P;
  o.value = featAt(idx, 3).z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  var color = i.col;
  var alpha = color.a * clamp(i.halfw - abs(i.across), 0.0, 1.0) * clamp((i.halfw - 1.0) * 2.0, 0.0, 1.0);
  let along = i.along;
  let time = frame.eye.w;
  let value = i.value;
  let position = i.world;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  // dashes, flowing along at u.b.y px a second (0: still)
  let dashP = max(u.a.z + u.a.w, 0.0001);
  let dashAt = i.alongPx - time * u.b.y;
  if (u.a.z > 0.0 && (dashAt - floor(dashAt / dashP) * dashP) > u.a.z) {
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
`,Fo=(e,t,n,r,i=`alpha`,a=!1)=>{let o=R({wgsl:Po,glsl:No},un.line,n,r,`layer '${t}'`);return new wn(e,{label:`line ${t}`,wgsl:o.wgsl,glsl:o.glsl,blocks:[`Frame`,`L`,`Params`],textures:[{name:`feat`,kind:`2d`,sample:`unfilterable`}],vertex:{stride:20,attributes:[{name:`a`,location:0,size:4,offset:0},{name:`b`,location:1,size:4,offset:4},{name:`p0`,location:2,size:4,offset:8},{name:`p3`,location:3,size:4,offset:12},{name:`x`,location:4,size:4,offset:16}]},blend:i??`alpha`,depth:{write:!1,test:a?`always`:`less`}})},Io=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e;var Lo=class e extends Ut{kind=`line`;mark=`line`;segs=new jo;inst=null;ibuf=null;cap=0;drapeable(){return!0}drapeMark(){return`line`}legacySits(){return this.props.over?`above`:void 0}get order(){return this.props.order??(this.sits===`above`||this.props.over&&!this.draped?4.5:void 0)}upperProg=null;upperFor=null;upperLast=!1;segTex=null;pieces=/* @__PURE__ */ new Float32Array;rangeJob=new Ee(()=>(this.attached?.world)?.invalidate?.());piecesOwn=!1;tileProg=null;tileOf=null;tilePieces=new d;geoGen=0;tileProgram(){return this.tileProg&&this.tileOf===this.progs||(this.tileProg?.destroy(),this.tileOf=this.progs,this.tileProg=zt(this.g,this.id,this.code(),this.paramBlock),this.tileProgramsMade([this.tileProg])),this.tileProg}animating(e){return super.animating(e)||this.rangeJob.running||this.flowing()}wantsFrames(e){return super.animating(e)||this.rangeJob.running?1/0:this.flowing()?Sn(Math.abs(this.props.flow)):0}flowing(){return this.flows()&&this.shown>0&&this.entries.length>0}flows(){let e=this.props;return!!e.dashes&&e.dashes[0]>0&&!!e.flow}selfMoving(){return super.selfMoving()||this.flows()}rangesStale(){return this.rangeJob.busy||!this.piecesOwn}wantRanges(){return!this.onTiles&&this.relief()}programs(e){return this.draped?[de(this.g,this.id,e,this.paramBlock,this.props.blend),de(this.g,this.id,e,this.paramBlock,this.props.blend,!0)]:[Fo(this.g,this.id,e,this.paramBlock,this.props.blend,this.props.over||this.sits===`above`)]}upper(){return this.upperProg&&this.upperFor===this.progs?this.upperProg:(this.upperProg?.destroy(),this.upperFor=this.progs,this.upperProg=de(this.g,this.id,this.code(),this.paramBlock,this.props.blend,!1,!0))}styleProps(){return[this.props.color,Ot(this.props.width),this.props.value]}sizeUnits(){return qt(this.props.width,`${this.mark} '${this.id}' width`)}style(e,t){let n=this.st;return this.colourInto(Io(this.props.color,e.d,t,`#ffb000`)),n.number=Io(Ot(this.props.width),e.d,t,2),n.value=Io(this.props.value,e.d,t,0),n}job=null;static buildSliceMs=8;build(t){if(this.job&&this.job.of!==this.entries&&(this.job=null),!this.job){let e=new jo;this.job={of:this.entries,next:e,run:this.startBuild(e)}}if(!this.job.run.run(e.buildSliceMs))return!1;this.segs=this.job.next,this.job=null,this.built2(),this.boxes(),this.upload(t)}startBuild(e){let t=typeof this.props.reveal==`object`?this.props.reveal.from:void 0,n=[];for(let e of this.entries){let r=e.geometry,i=r?.type===`Polygon`||r?.type===`MultiPolygon`,a=e.cuts?.flat(),o=z(r);for(let r=0;r<o.length;r++){let s=o[r],c=!1;if(t&&s.length>1){let e=s[0],n=s[s.length-1];c=(n[0]-t[0])**2+(n[1]-t[1])**2<(e[0]-t[0])**2+(e[1]-t[1])**2}n.push({f:e.index,line:s,closed:i,cuts:a?.[r],reverse:c,along:this.alongOf(e,r)})}}return new Ao(e,this.origin,n,this.step,this.draped?{form:`pieces`,row:ot*4}:{})}alongOf(e,t){}built2(){}upload(e){let t=this.segs.count;if(this.draped){if(!t)return;this.pieces=this.segs.pieces,this.groundRange(e,this.segs.lonlat),h(this.pieces,t,20,18,this.range),this.piecesOwn=!1,this.wantRanges()&&this.rangeJob.start(),this.segTex=r(e.graphics,this.segTex,this.pieces,`${this.id} segments`),this.geoGen++,this.lookGen++}else(!this.inst||this.cap<t)&&(this.inst?.destroy(),this.ibuf?.destroy(),this.cap=Math.max(64,this.cap?Math.ceil(t*1.25):t),this.inst=e.graphics.buffer(`vertex`,this.cap*20*4*4,`${this.id} segments`),this.ibuf=e.graphics.buffer(`index`,this.cap*6*4,`${this.id} segment triangles`)),t&&(this.inst.write(this.segs.data),this.ibuf.write(this.segs.index))}ground(e){this.draped?(this.groundRange(e,this.segs.lonlat),this.segs.count&&this.pieces.length>=this.segs.count*20&&this.wantRanges()?this.rangeJob.start():this.piecesOwn=!1):(this.segs.ground((t,n)=>this.groundAt(e,t,n)),this.upload(e))}uniforms(e){let t=this.props,n=t.dashes;this.L.set([1,1,n?.[0]??0,n?.[1]??0],28),this.L.set([t.glow??.5,n?t.flow??0:0,t.depthBias??2,0],32),this.L.set([0,0,0,0],36)}draws(e){if(this.draped&&!this.piecesOwn&&!this.rangeJob.busy&&this.segs.count&&this.wantRanges()&&this.rangeJob.start(),this.draped&&this.rangeJob.step(e.world.rangeIn?.bind(e.world),this.segs.count,It(this.segs),this.pieces,20,18,this.range)&&(this.segTex=r(e.graphics,this.segTex,this.pieces,`${this.id} segments`),this.piecesOwn=!this.rangeJob.busy),this.draped){if(!this.segTex||!this.segs.count)return[];let t={...this.call(this.progs[+!!this.flatNow],{count:this.segs.count*(this.flatNow?6:36)},e.frame),textures:[this.feat,this.segTex]};return this.onTiles&&(this.tileProgram(),t.ground=this.groundDraw(`line`,this.L,this.lbuf,e.now,e.s,(e,t,n,r)=>this.tileDraws(e,t,n,r))),this.upperLast=this.upperNow(e),this.upperLast?[t,{...this.call(this.upper(),{count:this.segs.count*36},e.frame),textures:[this.feat,this.segTex],upper:this.sits!==`ground`||`ground`,pickProgram:void 0}]:[t]}return!this.inst||!this.segs.count?[]:[this.call(this.progs[0],{vertices:this.inst,index:this.ibuf,count:this.segs.count*6},e.frame)]}preparePrograms(){this.onTiles&&this.tileProgram(),this.draped&&this.solidsAround()&&this.upper()}graphicsBusy(){let e=this.upperProg;return super.graphicsBusy()||this.onTiles&&!this.tileReady||this.upperLast&&!!e&&!e.ok&&!e.failed}warm(){this.onTiles&&this.g&&(this.tileProgram(),this.segs.count&&this.tilePieces.prepare(this.geoGen,this.segs.count,It(this.segs)))}tileDraws(e,t,n,r){let i=this.g,a=this.segs;if(!i||!this.segTex||!this.feat||!a.count)return;this.tilePieces.prepare(this.geoGen,a.count,It(a));let o=this.tilePieces.select(i,e.key,e.box,.06);o.n&&r.push({program:this.tileProgram(),blocks:[t,n,this.pbuf,e.block],textures:[this.feat,this.segTex,this.tilePieces.texture],count:o.n*6,first:o.first*6})}freeGeometry(){this.job=null,this.inst?.destroy(),this.ibuf?.destroy(),this.segTex?.destroy(),this.inst=this.ibuf=null,this.segTex=null,this.cap=0,this.tilePieces.destroy(),this.tileProg?.destroy(),this.tileProg=null,this.upperProg?.destroy(),this.upperProg=null,this.upperFor=null}readouts(){return{count:()=>this.size,segments:()=>this.segs.count}}};const Ro=e=>new Lo(e),zo=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e,Bo=e=>e.length>1&&e[0][0]===e[e.length-1][0]&&e[0][1]===e[e.length-1][1]?e.length-1:e.length;function Vo(e,t,n,r){let i=n.length/2,a=r.room(i*2),o=0;for(let e of t)o+=Bo(e);if(o===i)for(let e of t)for(let t=0,n=Bo(e);t<n;t++)a[r.n++]=e[t][0],a[r.n++]=e[t][1];else for(let t=0;t<i;t++)a[r.n++]=e.lonOf(n[2*t]),a[r.n++]=e.latOf(n[2*t+1])}var Ho=class extends Ut{kind=`area`;mark=`area`;lonlat=/* @__PURE__ */ new Float64Array;verts=/* @__PURE__ */ new Float32Array;vbuf=null;ibuf=null;nIdx=0;segs=new jo;sbuf=null;sibuf=null;scap=0;lbuf2=null;L2=/* @__PURE__ */ new Float32Array(64);drapeable(){return this.props.height===void 0||this.props.height===0}drapeMark(){return`area`}imaged(){return!1}imageTex=null;withPic(e){return this.imaged()&&this.imageTex?[...e,this.imageTex]:e}upperProgs=null;upperFor=null;upperLast=!1;uppers(){if(this.upperProgs&&this.upperFor===this.progs)return this.upperProgs;this.upperProgs?.forEach(e=>e.destroy()),this.upperFor=this.progs;let e=this.code();return this.upperProgs=[Mn(this.g,this.id,e,this.paramBlock,this.props.blend,!1,!0,this.imaged()),de(this.g,`${this.id} outline`,{functions:e.functions},this.paramBlock,this.props.blend,!1,!0)]}planesWanted(){return super.planesWanted()|16}triTex=null;segTex=null;nTri=0;tri=/* @__PURE__ */ new Float32Array;triBox=/* @__PURE__ */ new Float64Array;segPieces=/* @__PURE__ */ new Float32Array;ask=()=>(this.attached?.world)?.invalidate?.();triJob=new Ee(this.ask);segJob=new Ee(this.ask);triOwn=!1;segOwn=!1;tileProgs=null;tileOf=null;triPieces=new d;segPieces2=new d;geoGen=0;outGen=0;tilePrograms(){if(this.tileProgs&&this.tileOf===this.progs)return this.tileProgs;this.tileProgs?.forEach(e=>e.destroy()),this.tileOf=this.progs;let e=this.code();return this.tileProgs=[ke(this.g,this.id,e,this.paramBlock,this.imaged()),zt(this.g,`${this.id} outline`,{functions:e.functions},this.paramBlock)],this.tileProgramsMade(this.tileProgs),this.tileProgs}preparePrograms(){this.onTiles&&this.tilePrograms(),this.draped&&this.solidsAround()&&this.uppers()}graphicsBusy(){return super.graphicsBusy()||this.onTiles&&!this.tileReady||this.upperLast&&!!this.upperProgs?.some(e=>!e.ok&&!e.failed)}triBoxOf=(e,t)=>{let n=this.triBox;t[0]=n[4*e],t[1]=n[4*e+1],t[2]=n[4*e+2],t[3]=n[4*e+3]};wantRanges(){return!this.onTiles&&this.relief()}rangesStale(){return this.triJob.busy||!this.triOwn}stepRanges(e){if(this.wantRanges()&&(!this.triOwn&&!this.triJob.busy&&this.nTri&&this.triJob.start(),this.outlineKey===`*`&&!this.segOwn&&!this.segJob.busy&&this.segs.count&&this.segJob.start()),!this.triJob.busy&&!this.segJob.busy)return;let t=e.world.rangeIn?.bind(e.world);this.triJob.step(t,this.nTri,this.triBoxOf,this.tri,12,8,this.range)&&this.nTri&&(this.triTex=r(e.graphics,this.triTex,this.tri,`${this.id} triangles`),this.triOwn=!this.triJob.busy),!this.triJob.busy&&this.segJob.step(t,this.segs.count,It(this.segs),this.segPieces,20,18,this.range)&&this.segs.count&&(this.segTex=r(e.graphics,this.segTex,this.segPieces,`${this.id} outline`),this.segOwn=!this.segJob.busy)}animating(e){return super.animating(e)||this.triJob.running||this.segJob.running}programs(e){if(this.draped){let t={functions:e.functions};return[Mn(this.g,this.id,e,this.paramBlock,this.props.blend,!1,!1,this.imaged()),de(this.g,`${this.id} outline`,t,this.paramBlock,this.props.blend),Mn(this.g,this.id,e,this.paramBlock,this.props.blend,!0,!1,this.imaged()),de(this.g,`${this.id} outline`,t,this.paramBlock,this.props.blend,!0)]}let t=R(re({wgsl:Ie,glsl:Ne},1,this.imaged()),un.area,e,this.paramBlock,`layer '${this.id}'`);return[new wn(this.g,{label:`area ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`L`,`Params`],textures:[{name:`feat`,kind:`2d`,sample:`unfilterable`},...this.imaged()?[Fn]:[]],vertex:{stride:4,attributes:[{name:`v`,location:0,size:4,offset:0}]},blend:this.props.blend??`alpha`,depth:{write:!1,test:this.sits===`above`?`always`:`less`}}),Fo(this.g,`${this.id} outline`,{functions:e.functions},this.paramBlock,this.props.blend,this.sits===`above`)]}styleProps(){return[this.props.color,this.props.height,this.props.value]}style(e,t){let n=this.st;return this.colourInto(zo(this.props.color,e.d,t,`#4e79a7cc`)),n.number=zo(this.props.height,e.d,t,0),n.value=zo(this.props.value,e.d,t,0),n}build(e){if(this.outlineKey=``,this.outlineOf=null,this.draped)return this.buildDraped(e);let t=this.origin,n=new P(/* @__PURE__ */ new Float64Array(1024)),r=new P(/* @__PURE__ */ new Float32Array(2048)),i=new P(/* @__PURE__ */ new Uint32Array(1536));for(let e of this.entries)for(let a of Xe(e.geometry)){let o=ee(t,a,this.step),s=o.xy.length/2,c=r.n/4;Vo(t,a,o.xy,n);let l=r.room(s*4);for(let t=0;t<s;t++)l[r.n++]=o.xy[2*t],l[r.n++]=o.xy[2*t+1],l[r.n++]=0,l[r.n++]=e.index;let u=o.tris,d=i.room(u.length);for(let e=0;e<u.length;e++)d[i.n++]=c+u[e]}this.lonlat=n.done(),this.verts=r.done(),this.nIdx=i.n,this.vbuf?.destroy(),this.ibuf?.destroy(),this.vbuf=this.ibuf=null,this.nIdx&&(this.vbuf=e.graphics.buffer(`vertex`,this.verts,`${this.id} vertices`),this.ibuf=e.graphics.buffer(`index`,i.done(),`${this.id} triangles`)),this.lbuf2??=e.graphics.buffer(`uniform`,256,`${this.id} outline layer`),this.boxes()}job=null;buildDraped(e){if(this.job&&this.job.of!==this.entries&&(this.job=null),!this.job){let e=0,t=0;for(let n of this.entries)for(let r of Xe(n.geometry)){let n=0;for(let e of r)n+=Bo(e);e+=n,t+=Math.max(0,n+2*(r.length-1)-2)}this.job={of:this.entries,i:0,ll:new P(new Float64Array(Math.max(64,e*2))),tri:new P(new Float32Array(Math.max(1,Math.ceil(t*12/(ot*4)))*ot*4)),boxes:new P(new Float64Array(Math.max(64,t*4)))}}let t=this.job,n=Math.max(this.step,2500),i=performance.now(),a=this.origin,{ll:o,tri:s,boxes:c}=t;for(;t.i<this.entries.length;t.i++){if(performance.now()-i>4)return!1;let e=this.entries[t.i];for(let t of Xe(e.geometry)){let r=ee(a,t,n),i=o.n/2,l=r.xy,u=r.tris;Vo(a,t,l,o);let d=o.a,f=s.room(u.length/3*12),p=c.room(u.length/3*4);for(let t=0;t+2<u.length;t+=3){let n=u[t],r=u[t+1],a=u[t+2],o=l[2*n],m=l[2*n+1],h=l[2*r],g=l[2*r+1],_=l[2*a],v=l[2*a+1],y=(h-o)*(v-m)-(g-m)*(_-o);if(y===0)continue;let b=s.n;f[b]=o,f[b+1]=m,y>0?(f[b+2]=h,f[b+3]=g,f[b+4]=_,f[b+5]=v):(f[b+2]=_,f[b+3]=v,f[b+4]=h,f[b+5]=g),f[b+6]=e.index,f[b+7]=0,f[b+8]=0,f[b+9]=0,f[b+10]=0,f[b+11]=0,s.n+=12;let x=2*(i+n),S=2*(i+r),C=2*(i+a),w=c.n;p[w]=Math.min(d[x],d[S],d[C]),p[w+1]=Math.min(d[x+1],d[S+1],d[C+1]),p[w+2]=Math.max(d[x],d[S],d[C]),p[w+3]=Math.max(d[x+1],d[S+1],d[C+1]),c.n+=4}}}this.job=null,this.lonlat=o.done(),this.nTri=s.n/12,this.triBox=c.done();let l=Math.max(1,Math.ceil(s.n/(ot*4)))*ot*4;s.a.length===l?this.tri=s.a:(this.tri=new Float32Array(l),this.tri.set(s.a.subarray(0,s.n))),this.groundRange(e,this.lonlat),h(this.tri,this.nTri,12,8,this.range),this.triOwn=!1,this.nTri&&(this.triTex=r(e.graphics,this.triTex,this.tri,`${this.id} triangles`)),this.lbuf2??=e.graphics.buffer(`uniform`,256,`${this.id} outline layer`),this.geoGen++,this.lookGen++,this.boxes()}outlineKey=``;outlineOf=null;outlineWidths(){let e=this.props;return{own:typeof e.outline==`number`?e.outline:e.outline?.width??0,hover:e.hover?e.hover.outline??0:0,selected:e.selected===!1?0:e.selected?.outline??2}}outline(e){let t=this.outlineWidths(),n=t.own>0?null:this.litKeys(t.hover>0,t.selected>0),i=n===null?`*`:n.length?`+${n.join(``)}`:``;if(i===this.outlineKey&&this.outlineOf===this.entries)return;this.outlineKey=i,this.outlineOf=this.entries;let a=n===null?this.entries:n.map(e=>this.entryOf(e)).filter(Boolean),o=[];for(let e of a)Xe(e.geometry).forEach((t,n)=>t.forEach((t,r)=>o.push({f:e.index,line:t,closed:!0,cuts:e.cuts?.[n]?.[r]})));let s=this.draped;if(this.segs.build(this.origin,o,s?Math.max(this.step,2500):this.step,s?{form:`pieces`,row:ot*4}:{}),this.outGen++,this.segOwn=!1,s){this.segPieces=this.segs.pieces,h(this.segPieces,this.segs.count,20,18,this.range),(this.segs.count||this.onTiles)&&(this.segTex=r(e.graphics,this.segTex,this.segPieces,`${this.id} outline`));return}this.attached&&this.segs.ground((t,n)=>this.groundAt(e,t,n));let c=this.segs.count;c&&(!this.sbuf||this.scap<c)&&(this.sbuf?.destroy(),this.sibuf?.destroy(),this.scap=Math.max(64,c),this.sbuf=e.graphics.buffer(`vertex`,this.scap*20*4*4,`${this.id} outline`),this.sibuf=e.graphics.buffer(`index`,this.scap*6*4,`${this.id} outline triangles`)),c&&(this.sbuf.write(this.segs.data),this.sibuf.write(this.segs.index))}ground(e){if(this.draped){this.groundRange(e,this.lonlat),this.wantRanges()?(this.triJob.start(),this.outlineKey===`*`&&this.segJob.start()):(this.triOwn=!1,this.segOwn=!1);return}let t=this.verts,n=this.lonlat;for(let r=0,i=t.length/4;r<i;r++)t[r*4+2]=this.groundAt(e,n[2*r],n[2*r+1]);this.vbuf?.write(t),this.segs.count&&(this.segs.ground((t,n)=>this.groundAt(e,t,n)),this.sbuf?.write(this.segs.data))}uniforms(){let e=this.props,t=typeof e.outline==`number`?{width:e.outline}:e.outline??{},n=L(t.color??`#ffffff59`);this.L.set([e.depthBias??1.5,1,0,0],28),this.L.set([0,0,0,0],32),this.L.set([0,0,0,0],36),this.L2.set(this.L),this.L2.set([t.width??0,0,0,0],28),this.L2.set([0,0,(e.depthBias??1.5)+.5,1],32),this.L2.set(n,36),this.L2[43]=this.outlineKey===`*`&&this.segOwn&&!this.segJob.busy?0:1,this.lbuf2?.write(this.L2)}draws(e){let t=[];if(this.outline(e),this.draped){this.stepRanges(e),this.onTiles&&this.tilePrograms();let n=this.flatNow?2:0;if(this.triTex&&this.nTri){let r={...this.call(this.progs[n],{count:this.nTri*(this.flatNow?3:24)},e.frame),textures:this.withPic([this.feat,this.triTex])};this.onTiles&&(r.ground=this.groundDraw(`fill`,this.L,this.lbuf,e.now,e.s,(e,t,n,r)=>this.fillTile(e,t,n,r))),t.push(r)}let i=this.outlineWidths(),a=this.onTiles&&(i.own>0||i.hover>0||i.selected>0);if(a&&!this.segTex&&(this.segTex=r(e.graphics,null,this.segPieces.length?this.segPieces:new Float32Array(ot*4),`${this.id} outline`)),this.segTex&&this.lbuf2&&(this.segs.count&&this.outlineKey||a)){let r={...this.call(this.progs[n+1],{count:this.segs.count*(this.flatNow?6:36)},e.frame,this.lbuf2),textures:[this.feat,this.segTex]};this.onTiles&&(r.ground=this.groundDraw(`outline`,this.L2,this.lbuf2,e.now,e.s,(e,t,n,r)=>this.outlineTile(e,t,n,r))),t.push(r)}if(this.upperLast=this.upperNow(e),this.upperLast){let[n,r]=this.uppers(),i=this.sits!==`ground`||`ground`;this.triTex&&this.nTri&&t.push({...this.call(n,{count:this.nTri*24},e.frame),textures:this.withPic([this.feat,this.triTex]),upper:i,pickProgram:void 0}),this.segTex&&this.lbuf2&&this.segs.count&&this.outlineKey&&t.push({...this.call(r,{count:this.segs.count*36},e.frame,this.lbuf2),textures:[this.feat,this.segTex],upper:i,pickProgram:void 0})}return t}if(this.vbuf&&this.ibuf&&this.nIdx){let n=this.call(this.progs[0],{vertices:this.vbuf,index:this.ibuf,count:this.nIdx},e.frame);t.push({...n,textures:this.withPic(n.textures??[])})}return this.sbuf&&this.sibuf&&this.segs.count&&this.lbuf2&&this.outlineKey&&t.push(this.call(this.progs[1],{vertices:this.sbuf,index:this.sibuf,count:this.segs.count*6},e.frame,this.lbuf2)),t}warm(){this.onTiles&&this.g&&(this.tilePrograms(),this.nTri&&this.triPieces.prepare(this.geoGen,this.nTri,this.triBoxOf))}fillTile(e,t,n,r){let i=this.g;if(!i||!this.triTex||!this.feat||!this.nTri)return;this.triPieces.prepare(this.geoGen,this.nTri,this.triBoxOf);let a=this.triPieces.select(i,e.key,e.box,0);a.n&&r.push({program:this.tilePrograms()[0],blocks:[t,n,this.pbuf,e.block],textures:this.withPic([this.feat,this.triTex,this.triPieces.texture]),count:a.n*3,first:a.first*3})}outlineTile(e,t,n,r){let i=this.g,a=this.segs;if(!i||!this.segTex||!this.feat||!a.count)return;this.segPieces2.prepare(this.outGen,a.count,It(a));let o=this.segPieces2.select(i,e.key,e.box,.06);o.n&&r.push({program:this.tilePrograms()[1],blocks:[t,n,this.pbuf,e.block],textures:[this.feat,this.segTex,this.segPieces2.texture],count:o.n*6,first:o.first*6})}pickBoxes=null;pickOf=null;groundPick(e,t){if(!this.onTiles||this.shown<=0)return null;let n=this.entries;if(this.pickOf!==n){let e=new Float64Array(n.length*4);n.forEach((t,n)=>{let r=O(t.geometry??null);r?e.set(r,n*4):e.set([1,1,-1,-1],n*4)}),this.pickBoxes=e,this.pickOf=n}let r=this.pickBoxes;for(let i=n.length-1;i>=0;i--)if(!(e<r[4*i]||t<r[4*i+1]||e>r[4*i+2]||t>r[4*i+3])&&this.isShown(n[i].index)&&dn(n[i].geometry??null,[e,t]))return n[i].index;return null}freeGeometry(){this.upperProgs?.forEach(e=>e.destroy()),this.upperProgs=null,this.upperFor=null,this.triPieces.destroy(),this.segPieces2.destroy(),this.tileProgs?.forEach(e=>e.destroy()),this.tileProgs=null;for(let e of[this.vbuf,this.ibuf,this.sbuf,this.sibuf,this.lbuf2,this.triTex,this.segTex])e?.destroy();this.vbuf=this.ibuf=this.sbuf=this.sibuf=this.lbuf2=null,this.triTex=this.segTex=null,this.scap=0,this.outlineKey=``}readouts(){return{count:()=>this.size,triangles:()=>this.draped?this.nTri:this.nIdx/3,outline:()=>this.segs.count}}};const Uo=e=>new Ho(e),Wo=`${Be}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
bool behindGlobe(vec3 p) {
  return frame.size.w < 0.5 && dot(p - vec3(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
`,Go={vertex:`${Wo}
layout(location = 0) in vec4 at;
out vec2 v_corner;
out vec4 v_col;
out vec3 v_world;
out vec4 v_extra;
out float v_size;
out float v_reveal;
out float v_ring;
vec3 towardEye(vec3 p, float discPx) {
  vec3 toEye = frame.eye.xyz - p;
  float d = length(toEye);
  float mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(discPx * frame.size.z * 0.5 * mpp, d * 0.5);
}
vec2 cornerOf(int v) {
  float x = (v == 1 || v == 4 || v == 5) ? 1.0 : -1.0;
  float y = (v == 2 || v == 3 || v == 5) ? 1.0 : -1.0;
  return vec2(x, y);
}
void main() {
  float idx = at.w;
  vec4 state = featAt(idx, 2);
  v_corner = vec2(0.0);
  v_col = vec4(0.0);
  v_world = vec3(0.0);
  v_extra = vec4(0.0);
  v_size = 1.0;
  v_reveal = 0.0;
  v_ring = 0.0;
  if (state.w <= 0.0) {
    gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
    return;
  }
  vec4 nums = featAt(idx, 3);
  vec2 corner = cornerOf(gl_VertexID);
  vec3 position = placeAt(at.xy, at.z * u.origin.z + u.origin.w);
  bool hide = behindGlobe(position);
  float hov = state.x;
  float sel = state.y;
  float ring = u.a.x + u.hover2.y * hov + u.sel2.y * sel;
  float grow = 1.0;
  float reveal = state.z;
  if (u.reveal.x == 8.0) {
    grow = max(0.0, easeOutBack(state.z));
    reveal = min(1.0, state.z * 4.0);
  }
  float size = sizePx(mix(nums.y, nums.x, u.look.z), u.size, position) * grow * mix(1.0, u.a.z, hov) * mix(1.0, u.a.w, sel) + 2.0 * (ring - u.a.x);
  float value = nums.z;
  float time = frame.eye.w;
  /*@position*/
  vec3 lifted = towardEye(position, size);
  vec4 clip = frame.viewProj * vec4(lifted, 1.0);
  vec2 off = corner * size * frame.size.z / frame.size.xy * clip.w;
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(vec4(clip.xy + off, clip.z - u.a.y * clip.w, clip.w));
  v_corner = corner;
  v_world = position;
  v_extra = vec4(value, state.w, idx, hov + 2.0 * sel);
  v_size = size * frame.size.z;
  v_reveal = reveal;
  v_ring = ring;
  v_col = colourOf(idx, state);
}
`,fragment:`${Wo}
in vec2 v_corner;
in vec4 v_col;
in vec3 v_world;
in vec4 v_extra;
in float v_size;
in float v_reveal;
in float v_ring;
out vec4 outColor;
void main() {
  float r = length(v_corner);
  float px = 2.0 / max(v_size, 1.0);
  float disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
  float ring = smoothstep(1.0 - px * (v_ring + 1.0), 1.0 - px * v_ring, r);
  float hov = (v_extra.w == 1.0 || v_extra.w == 3.0) ? 1.0 : 0.0;
  float sel = v_extra.w >= 2.0 ? 1.0 : 0.0;
  vec3 oc = mix(mix(u.b.rgb, u.sel.rgb, sel), u.hover.rgb, hov);
  float oa = max(u.b.a, max(hov, sel));
  vec4 color = vec4(mix(v_col.rgb, oc, ring * oa), v_col.a * disc);
  vec2 uv = v_corner;
  float value = v_extra.x;
  float time = frame.eye.w;
  vec3 position = v_world;
  /*@color*/
  float alpha = color.a * u.look.x * v_extra.y;
  /*@alpha*/
  float reveal = v_reveal;
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.hover2.w, v_extra.z);
    return;
  }
  outColor = vec4(themed(color.rgb, max(hov, sel)), alpha);
}
`},Ko=`${A}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct I { @builtin(vertex_index) vid: u32, @location(0) at: vec4f }
struct O { @builtin(position) pos: vec4f, @location(0) corner: vec2f, @location(1) col: vec4f, @location(2) world: vec3f, @location(3) extra: vec4f, @location(4) size: f32, @location(5) reveal: f32, @location(6) ring: f32 }
/*@functions*/
// on the far side of the globe (the earth's middle is RE under the view centre)
fn behindGlobe(p: vec3f) -> bool {
  return frame.size.w < 0.5 && dot(p - vec3f(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
// moved along the line to the eye by half the disc: the same place on screen, and the disc stands in front of the
// ground under it instead of half into it
fn towardEye(p: vec3f, discPx: f32) -> vec3f {
  let toEye = frame.eye.xyz - p;
  let d = length(toEye);
  let mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(discPx * frame.size.z * 0.5 * mpp, d * 0.5);
}
fn cornerOf(v: i32) -> vec2f {
  let x = select(-1.0, 1.0, v == 1 || v == 4 || v == 5);
  let y = select(-1.0, 1.0, v == 2 || v == 3 || v == 5);
  return vec2f(x, y);
}
@vertex fn vs(i: I) -> O {
  var o: O;
  let idx = i.at.w;
  let state = featAt(idx, 2);
  // hidden (its own gate, a filter): nothing drawn
  if (state.w <= 0.0) {
    o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
    return o;
  }
  let nums = featAt(idx, 3);
  let corner = cornerOf(i32(i.vid));
  var position = placeAt(i.at.xy, i.at.z * u.origin.z + u.origin.w);
  let hide = behindGlobe(position);
  let hov = state.x;
  let sel = state.y;
  let ring = u.a.x + u.hover2.y * hov + u.sel2.y * sel;
  // pop: the size overshoots as it grows in, the disc shows over the first quarter
  var grow = 1.0;
  var reveal = state.z;
  if (u.reveal.x == 8.0) {
    grow = max(0.0, easeOutBack(state.z));
    reveal = min(1.0, state.z * 4.0);
  }
  // its size: CSS px, or metres where it is seen, within the layer's range (core › render › drawing › size)
  var size = sizePx(mix(nums.y, nums.x, u.look.z), u.size, position) * grow * mix(1.0, u.a.z, hov) * mix(1.0, u.a.w, sel) + 2.0 * (ring - u.a.x);
  let value = nums.z;
  let time = frame.eye.w;
  /*@position*/
  let lifted = towardEye(position, size);
  let clip = frame.viewProj * vec4f(lifted, 1.0);
  let off = corner * size * frame.size.z / frame.size.xy * clip.w;
  o.pos = select(clipOut(vec4f(clip.xy + off, clip.z - u.a.y * clip.w, clip.w)), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.corner = corner;
  o.world = position;
  o.extra = vec4f(value, state.w, idx, hov + 2.0 * sel);
  o.size = size * frame.size.z;
  o.reveal = reveal;
  o.ring = ring;
  o.col = colourOf(idx, state);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let r = length(i.corner);
  let px = 2.0 / max(i.size, 1.0);
  let disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
  let ring = smoothstep(1.0 - px * (i.ring + 1.0), 1.0 - px * i.ring, r);
  let hov = select(0.0, 1.0, i.extra.w == 1.0 || i.extra.w == 3.0);
  let sel = select(0.0, 1.0, i.extra.w >= 2.0);
  let oc = mix(mix(u.b.rgb, u.sel.rgb, sel), u.hover.rgb, hov);
  let oa = max(u.b.a, max(hov, sel));
  var color = vec4f(mix(i.col.rgb, oc, ring * oa), i.col.a * disc);
  let uv = i.corner;
  let value = i.extra.x;
  let time = frame.eye.w;
  let position = i.world;
  /*@color*/
  var alpha = color.a * u.look.x * i.extra.y;
  /*@alpha*/
  let reveal = i.reveal;
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.hover2.w, i.extra.z);
  }
  return vec4f(themed(color.rgb, max(hov, sel)), alpha);
}
`,qo=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e;var Jo=class extends Ut{kind=`point`;mark=`point`;inst=null;data=/* @__PURE__ */ new Float32Array;cap=0;count=0;ll=/* @__PURE__ */ new Float64Array;own=/* @__PURE__ */ new Float32Array;fi=/* @__PURE__ */ new Float32Array;paths=[];constructor(e){if(super(e),e.at!==void 0&&typeof e.at!=`function`)throw new K(`E-DATA-001`,`points '${this.id}': at must be a function d => [lon, lat, h?]`,{where:this.id,fix:`at: d => d.at`})}drapeMark(){return`point`}programs(e){let t=R({wgsl:Ko,glsl:Go},un.points,e,this.paramBlock,`layer '${this.id}'`),n={label:`points ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`L`,`Params`],textures:[{name:`feat`,kind:`2d`,sample:`unfilterable`}],instance:{stride:4,attributes:[{name:`at`,location:0,size:4,offset:0}]},blend:`alpha`,depth:{write:!1,test:`less`}},r=new wn(this.g,n);return this.sits===`above`&&(this.props.behind??.35)>0?[r,new wn(this.g,{...n,label:`points ${this.id} (behind)`,depth:{write:!1,test:`greater-equal`}})]:[r]}lbufB=null;LB=/* @__PURE__ */ new Float32Array(64);read(e){let t=super.read(e),n=this.props.at;if(n)for(let e of t){let t=n(e.d);e.geometry={type:`Point`,coordinates:[t[0],t[1],...t[2]===void 0?[]:[t[2]]]}}return t}styleProps(){return[this.props.color,Ot(this.props.size),this.props.value]}sizeUnits(){return qt(this.props.size,`points '${this.id}' size`)}style(e,t){let n=e.d,r=this.st;return this.colourInto(qo(this.props.color,e.d,t,`#ffb000`)),r.number=qo(Ot(this.props.size),e.d,t,8),r.value=qo(this.props.value,e.d,t,typeof n?.value==`number`?n.value:0),r}build(){this.paths=[];let e=this.props.along,t=this.entries;if(e){let n=[],r=[];for(let i of t){let t=i.geometry?.type===`LineString`?i.geometry.coordinates:i.geometry?.type===`MultiLineString`?i.geometry.coordinates.flat():null,a=e.times(i.d);if(!t?.length||!a?.length)continue;let o=Math.min(t.length,a.length),s=new Float64Array(o*2),c=new Float64Array(o);for(let e=0;e<o;e++){s[2*e]=t[e][0],s[2*e+1]=t[e][1];let n=a[e];c[e]=typeof n==`number`?n:n.getTime()}n.push(s[0],s[1]),r.push(i.index),this.paths.push({ll:s,t:c})}this.ll=Float64Array.from(n),this.fi=Float32Array.from(r),this.own=new Float32Array(r.length).fill(NaN),this.boxes();return}let n=0;for(let e of t){let t=e.geometry;n+=t?.type===`Point`?1:t?.type===`MultiPoint`?t.coordinates.length:+!!H(t)}let r=new Float64Array(n*2),i=new Float32Array(n),a=new Float32Array(n),o=0;for(let e of t){let t=e.geometry,n=t?.type===`Point`||t?.type===`MultiPoint`?We(t):null,s=t=>{r[2*o]=t[0],r[2*o+1]=t[1],i[o]=t[2]===void 0?NaN:t[2],a[o]=e.index,o++};if(n)for(let e of n)s(e);else{let e=H(t);e&&s(e)}}this.ll=r,this.own=i,this.fi=a,this.boxes()}ground(){}hts=/* @__PURE__ */ new Float32Array;hVer=-1;hAt=-1e9;key=``;hNext=-1;hPass=-1;uniforms(e){let t=this.props,n=this.L,r=t.hover===!1?null:t.hover??(t.tooltip||t.onHover?{color:`#ffffff`}:null),i=t.selected===!1?null:t.selected??{color:`#ffd166`,outline:2},a=L(r?.color??`#ffffff`),o=L(i?.color??`#ffd166`),s=L(t.outline?.color??`#00000080`);n[2]=e.world.exaggeration,n[12]=a[0],n[13]=a[1],n[14]=a[2],n[15]=r?.mix??0,n[17]=r?r.outline??1.5:0,n[20]=o[0],n[21]=o[1],n[22]=o[2],n[23]=i?.mix??0,n[25]=i?i.outline??2:0,n[28]=t.outline?.width??1.5,n[29]=t.depthBias===void 0?2e-5:t.depthBias*1e-5,n[30]=r?1.25:1,n[31]=i?1.15:1,n[32]=s[0],n[33]=s[1],n[34]=s[2],n[35]=s[3],n[36]=n[37]=n[38]=n[39]=0}draws(e){let t=e.graphics,n=this.fi.length,r=this.props;if(!n||!this.feat)return[];(!this.inst||this.cap!==Math.max(64,n))&&(this.inst?.destroy(),this.cap=Math.max(64,n),this.inst=t.buffer(`instance`,this.cap*4*4,`${this.id} instances`),this.data=new Float32Array(this.cap*4),this.key=``);let i=r.on??`ground`,a=this.paths.length>0,o=e.world.groundVersion,s=this.ll,c=this.own,l=t=>{let n=c[t],r=n===n?n:i===`ground`?e.world.heightAt(s[2*t],s[2*t+1]):0;this.hts[t]=Number.isNaN(r)?0:r};if(this.hts.length!==n){if(this.hts=new Float32Array(n),!a)for(let e=0;e<n;e++)l(e);this.hVer=o,this.hAt=e.now,this.hNext=-1,this.key=``}else!a&&this.hNext<0&&o!==this.hVer&&e.now-this.hAt>250&&(this.hNext=0,this.hPass=o);if(!a&&this.hNext>=0){let t=performance.now();for(;this.hNext<n&&(l(this.hNext++),!(!(this.hNext&255)&&performance.now()-t>2)););this.hNext>=n&&(this.hNext=-1,this.hVer=this.hPass,this.hAt=e.now,this.key=``)}let u=`${this.hVer}|${n}|${this.entries.length}`;if(a||u!==this.key||this.placed!==s){this.key=u,this.placed=s;let t=this.data,i=this.origin,o=a?(()=>{let t=r.along.clock?r.along.clock(e.s):e.s.time.now;return typeof t==`number`?t:t.getTime()})():0;for(let e=0;e<n;e++){let n=this.paths[e];if(n){let t=this.whereAt(n,o);s[2*e]=t[0],s[2*e+1]=t[1],l(e)}let r=e*4;t[r]=i.x(s[2*e]),t[r+1]=i.y(s[2*e+1]),t[r+2]=this.hts[e],t[r+3]=this.fi[e]}this.count=n,this.inst.write(t)}let d=this.progs[0],f=[{...this.call(d,{instances:this.inst,count:6,instanceCount:this.count},e.frame)}],p=this.progs[1];return p&&(this.lbufB??=t.buffer(`uniform`,256,`${this.id} layer (behind)`),this.LB.set(this.L),this.LB[4]=this.L[4]*(r.behind??.35),this.lbufB.write(this.LB),f.push({...this.call(p,{instances:this.inst,count:6,instanceCount:this.count},e.frame,this.lbufB)})),f}placed=null;at2=[0,0];whereAt(e,t){let n=e.t,r=n.length,i=this.at2;if(t<=n[0]||r===1)return i[0]=e.ll[0],i[1]=e.ll[1],i;if(t>=n[r-1])return i[0]=e.ll[2*r-2],i[1]=e.ll[2*r-1],i;let a=0,o=r-1;for(;o-a>1;){let e=a+o>>1;n[e]<=t?a=e:o=e}let s=(t-n[a])/Math.max(1e-9,n[o]-n[a]);return i[0]=e.ll[2*a]+(e.ll[2*o]-e.ll[2*a])*s,i[1]=e.ll[2*a+1]+(e.ll[2*o+1]-e.ll[2*a+1])*s,i}animating(e){return this.hNext>=0||!!this.props.along&&(!!this.props.along.clock||this.attached?.world?.timeRunning===!0)}status(){let e=super.status();return this.hNext>=0&&(e.pending.push(`reading the ground under its places`),e.ready=!1),e}freeGeometry(){this.inst?.destroy(),this.inst=null,this.cap=0,this.placed=null,this.lbufB?.destroy(),this.lbufB=null}readouts(){return{count:()=>this.size,shown:()=>{let e=0;for(let t=0;t<this.count;t++)this.featData[this.at(this.fi[t],2)+3]>0&&e++;return e}}}};const Yo=e=>new Jo(e),Xo=`${Be}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,Zo={vertex:`${Xo}
layout(location = 0) in vec4 v;
layout(location = 1) in vec4 n;
out vec4 v_col;
flat out float v_idx;
flat out vec4 v_state;
out vec3 v_world;
out vec3 v_normal;
out float v_up;
flat out float v_value;
out float v_shade;
float shade(vec3 nn, float top) {
  vec3 l = normalize(vec3(-0.45, 0.8, -0.35));
  return mix(0.62 + 0.38 * max(dot(nn, l), 0.0), 1.0, top * 0.35);
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
  float ex = u.origin.z;
  float grow = 1.0;
  float scale = motion.w;
  if (u.reveal.x == 9.0) {
    grow = state.z;
  }
  if (u.reveal.x == 8.0) {
    scale = scale * easeOutBack(state.z);
    grow = easeOutBack(state.z);
  }
  vec2 rel = box.xy + (v.xy - box.xy) * scale + vec2(motion.x, motion.y) / (RE * max(frame.misc.y, 0.01));
  float height = mix(nums.y, nums.x, u.look.z) * grow;
  float base = nums.w * ex + u.origin.w + motion.z + raiseOf(state);
  float h = min(v.z, nums.w) * ex - 30.0;
  if (n.w > 0.5) {
    h = base + max(height, 0.0);
  }
  float value = nums.z;
  float time = frame.eye.w;
  vec3 position = placeAt(rel, h);
  /*@position*/
  gl_Position = clipOut(frame.viewProj * vec4(liftToEye(position, u.a.x), 1.0));
  vec4 c = colourOf(idx, state);
  v_col = c;
  v_shade = shade(n.xyz, n.w);
  v_idx = idx;
  v_state = state;
  v_world = position;
  v_normal = n.xyz;
  v_up = n.w;
  v_value = value;
}
`,fragment:`${Xo}
${Tn(1)}
in vec4 v_col;
flat in float v_idx;
flat in vec4 v_state;
in vec3 v_world;
in vec3 v_normal;
in float v_up;
flat in float v_value;
in float v_shade;
out vec4 outColor;
void main() {
  vec4 color = vec4(airFog(themed(mix(v_col.rgb * v_shade, sunLit(v_col.rgb, normalize(v_normal), v_world), frame.sunColor.w), max(v_state.x, v_state.y)), v_world), v_col.a);
  float alpha = color.a;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  float time = frame.eye.w;
  float value = v_value;
  vec3 position = v_world;
  vec3 normal = v_normal;
  if (mode == 0.0 || mode == 7.0) {
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * step(0.001, reveal);
  }
  /*@color*/
  alpha = alpha * u.look.x * v_state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    outColor = packDepth(gl_FragCoord.z);
    return;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.hover2.w, v_idx);
    return;
  }
  outColor = vec4(color.rgb * mix(1.0, alpha, u.hover2.z), alpha);
}
`},Qo=`${A}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
${rt(1)}
struct I { @location(0) v: vec4f, @location(1) n: vec4f }
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec4f,
  @location(1) @interpolate(flat) idx: f32,
  @location(2) @interpolate(flat) state: vec4f,
  @location(3) world: vec3f,
  @location(4) normal: vec3f,
  @location(5) up: f32,
  @location(6) @interpolate(flat) value: f32,
  @location(7) shade: f32,
}
/*@functions*/
fn shade(n: vec3f, top: f32) -> f32 {
  let l = normalize(vec3f(-0.45, 0.8, -0.35));
  return mix(0.62 + 0.38 * max(dot(n, l), 0.0), 1.0, top * 0.35);
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
  let ex = u.origin.z;
  var grow = 1.0;
  var scale = motion.w;
  if (u.reveal.x == 9.0) {
    grow = state.z;
  }
  if (u.reveal.x == 8.0) {
    scale = scale * easeOutBack(state.z);
    grow = easeOutBack(state.z);
  }
  let rel = box.xy + (i.v.xy - box.xy) * scale + vec2f(motion.x, motion.y) / (RE * max(frame.misc.y, 0.01));
  let height = mix(nums.y, nums.x, u.look.z) * grow;
  let base = nums.w * ex + u.origin.w + motion.z + raiseOf(state);
  var h = min(i.v.z, nums.w) * ex - 30.0;
  if (i.n.w > 0.5) {
    h = base + max(height, 0.0);
  }
  let value = nums.z;
  let time = frame.eye.w;
  var position = placeAt(rel, h);
  /*@position*/
  var o: O;
  o.pos = clipOut(frame.viewProj * vec4f(liftToEye(position, u.a.x), 1.0));
  let c = colourOf(idx, state);
  o.col = c;
  o.shade = shade(i.n.xyz, i.n.w);
  o.idx = idx;
  o.state = state;
  o.world = position;
  o.normal = i.n.xyz;
  o.up = i.n.w;
  o.value = value;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  // lit by the world's sun and sky (with shadows), or the fixed shading
  var color = vec4f(airFog(themed(mix(i.col.rgb * i.shade, sunLit(i.col.rgb, normalize(i.normal), i.world), frame.sunColor.w), max(i.state.x, i.state.y)), i.world), i.col.a);
  var alpha = color.a;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  let time = frame.eye.w;
  let value = i.value;
  let position = i.world;
  let normal = i.normal;
  if (mode == 0.0 || mode == 7.0) {
    alpha = alpha * reveal;
  } else if (mode != 14.0) {
    alpha = alpha * step(0.001, reveal);
  }
  /*@color*/
  alpha = alpha * u.look.x * i.state.w;
  /*@alpha*/
  /*@reveal*/
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    return packDepth(i.pos.z);
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.hover2.w, i.idx);
  }
  return vec4f(color.rgb * mix(1.0, alpha, u.hover2.z), alpha);
}
`,$o=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e;var es=class extends Ut{kind=`extrusion`;mark=`extrusion`;planesWanted(){return super.planesWanted()|24}lonlat=/* @__PURE__ */ new Float64Array;corner=/* @__PURE__ */ new Int32Array;verts=/* @__PURE__ */ new Float32Array;feature=/* @__PURE__ */ new Int32Array;vbuf=null;n=0;programs(e){let t=R({wgsl:Qo,glsl:Zo},un.extrusion,e,this.paramBlock,`layer '${this.id}'`);return[new wn(this.g,{label:`extrusion ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`L`,`Params`],textures:[{name:`feat`,kind:`2d`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`}],vertex:{stride:8,attributes:[{name:`v`,location:0,size:4,offset:0},{name:`n`,location:1,size:4,offset:4}]},blend:this.props.blend??`alpha`,depth:{write:!0,test:`less`},cull:`none`})]}styleProps(){return[this.props.color,this.props.height,this.props.value]}style(e,t){return{color:L($o(this.props.color,e.d,t,`#d8d2c4`)),number:$o(this.props.height,e.d,t,10),value:$o(this.props.value,e.d,t,0)}}build(e){let t=[],n=[],r=[],i=[],a=/* @__PURE__ */ new Map,o=(e,o,s,c,l)=>{let u=`${e},${o}`,d=a.get(u);if(d===void 0){d=t.length/2,a.set(u,d);let n=this.origin.lonLat(e,o);t.push(n[0],n[1])}i.push(d),n.push(e,o,0,s,c[0],c[1],c[2],l),r.push(s)};for(let e of this.entries)for(let t of Xe(e.geometry)){let n=ee(this.origin,t,0xe8d4a51000);for(let t of n.tris)o(n.xy[2*t],n.xy[2*t+1],e.index,[0,1,0],1);t.forEach((t,n)=>{let r=t.map(e=>this.origin.rel(e[0],e[1])),i=0;for(let e=0,t=r.length-1;e<r.length;t=e++)i+=(r[t][0]-r[e][0])*(r[t][1]+r[e][1]);let a=i>0,s=n===0?a:!a;for(let t=0;t+1<r.length;t++){let n=r[t],i=r[t+1],a=i[0]-n[0],c=i[1]-n[1],l=Math.hypot(a,c)||1,u=[(s?c:-c)/l,0,-((s?-a:a)/l)];o(n[0],n[1],e.index,u,0),o(i[0],i[1],e.index,u,0),o(n[0],n[1],e.index,u,1),o(n[0],n[1],e.index,u,1),o(i[0],i[1],e.index,u,0),o(i[0],i[1],e.index,u,1)}})}this.lonlat=Float64Array.from(t),this.corner=Int32Array.from(i),this.verts=Float32Array.from(n),this.feature=Int32Array.from(r),this.n=r.length,this.vbuf?.destroy(),this.vbuf=this.n?e.graphics.buffer(`vertex`,this.verts,`${this.id} vertices`):null,this.boxes()}groundEvery=600;ground(e){let t=this.verts,n=this.lonlat,r=this.featData,i=new Float64Array(this.bySlot.length).fill(1/0),a=n.length/2,o=new Float64Array(a);for(let t=0;t<a;t++)o[t]=this.groundAt(e,n[2*t],n[2*t+1]);for(let e=0;e<this.n;e++){let n=o[this.corner[e]];t[e*8+2]=n;let r=this.feature[e];n<i[r]&&(i[r]=n)}for(let e of this.entries)r[this.at(e.index,3)+3]=Number.isFinite(i[e.index])?i[e.index]:0;this.vbuf?.write(t),this.touch(3),this.flushPlanes()}uniforms(){this.L.set([this.props.depthBias??.5,0,0,0],28),this.L.set([0,0,0,0],32),this.L.set([0,0,0,0],36)}draws(e){return!this.vbuf||!this.n?[]:[{...this.call(this.progs[0],{vertices:this.vbuf,count:this.n},e.frame),textures:[this.feat,e.shadowMap],cast:this.props.shadows!==!1}]}freeGeometry(){this.vbuf?.destroy(),this.vbuf=null}};const ts=e=>new es(e),ns=[`tooltip`,`hover`,`onTap`,`onHover`,`select`,`drape`,`subject`];var rs=class extends D{kind;signs=[];labels=null;constructor(e){if(!e||e.data===void 0)throw new K(`E-DATA-001`,`layer() needs data`,{fix:`layer({ id: 'suburbs', data: data.wfs(url, { typeName, extent }), area: { color }, labels: { text } })`});let t=e.area?.color??e.extrusion?.color??e.line?.color??e.points?.color;super({...e,color:t});let n=this.id,r={};for(let t of ns)e[t]!==void 0&&(r[t]=e[t]);let i=t=>(t??1)*(e.opacity??1),a=(t,a,o)=>{if(!t)return;let s=t,c=a({...r,...t,id:`${n}-${o}`,data:e.data,opacity:i(s.opacity)});c.signOpacity=s.opacity??1,c.owner=this,this.signs.push(c)};if(a(e.area,e=>Uo(e),`fill`),a(e.extrusion,e=>ts(e),`extrusion`),a(e.line,e=>Ro(e),`line`),a(e.points,e=>Yo(e),`points`),e.labels){let t=lt({halo:2,collide:!0,size:12,...r,...e.labels,id:`${n}-labels`,data:e.data,opacity:i(e.labels.opacity)});t.signOpacity=e.labels.opacity??1,t.owner=this,this.labels=t}if(!this.signs.length&&!this.labels)throw new K(`E-DATA-001`,`layer '${n}' says nothing: give it a sign`,{where:n,fix:`area: { color }, line: { color }, points: { size }, extrusion: { height } or labels: { text }`});this.kind=this.signs[0]?.kind??`label`}get source(){return(this.signs[0]??this.labels)?.source??null}get fades(){return!0}get sits(){return(this.signs[0]??this.labels).sits}get order(){return this.props.order??this.signs[0]?.order}get joined(){return(this.signs[0]??this.labels).joined}companions(){return this.labels?[this.labels]:[]}restacked(e){this.labels&&(this.labels.props={...this.labels.props,order:6+e,above:void 0,below:void 0})}async attach(e){await super.attach(e);for(let e of this.signs)e.pickNo=this.pickNo;await Promise.all(this.signs.map(t=>t.attach(e)))}detach(){for(let e of this.signs)e.detach();super.detach()}setProps(e){super.setProps(e);let t={};`data`in e&&(t.data=e.data);for(let n of ns)n in e&&(t[n]=e[n]);for(let n of[...this.signs,...this.labels?[this.labels]:[]]){let r={...t};`opacity`in e&&(r.opacity=(n.signOpacity??1)*(e.opacity??1)),Object.keys(r).length&&n.setProps(r)}let n=(t,n)=>{let r=e[t];if(!r)return;let i=n===`labels`?this.labels:this.signs.find(e=>e.id===`${this.id}-${n}`);i&&(r.opacity!==void 0&&(i.signOpacity=r.opacity),i.setProps({...r,...r.opacity===void 0?{}:{opacity:r.opacity*(this.props.opacity??1)}}))};return n(`area`,`fill`),n(`line`,`line`),n(`points`,`points`),n(`extrusion`,`extrusion`),n(`labels`,`labels`),this}frame(e){let t=[];for(let n of this.signs)n.shown=this.shown,n.target=this.target,n.pickNo=this.pickNo,t.push(...n.frame(e));return t}prepare(e){let t=!0;for(let n of this.signs)n.prepare(e)||(t=!1);return this.labels&&!this.labels.prepare(e)&&(t=!1),t}letGo(){for(let e of this.signs)e.letGo?.();this.labels?.letGo?.()}status(){let e={ready:!0,pending:[],failed:this.failure?[{what:this.id,error:this.failure.message}]:[],missing:[]};for(let t of this.signs){let n=t.status();e.ready&&=n.ready,e.pending.push(...n.pending),e.failed.push(...n.failed),e.missing.push(...n.missing)}return e}wantsFrames(e){let t=0;for(let n of this.signs)t=Math.max(t,n.wantsFrames(e));return t}animating(e){return this.signs.some(t=>t.animating(e))}revealSeconds(){return Math.max(0,...this.signs.map(e=>e.revealSeconds()))}pickInfo(e){for(let t of this.signs){let n=t.pickInfo(e);if(n)return{...n,element:this.id}}return null}groundPick(e,t){for(let n of this.signs){let r=n.groundPick?.(e,t);if(r!=null)return r}return null}hovered(e){for(let t of this.signs)t.hovered(e);this.labels?.hovered(e)}placeOf(e){for(let t of this.signs){let n=t.placeOf(e);if(n)return n}return null}attribution(){return(this.signs[0]??this.labels).attribution()}readouts(){let e=this.signs[0]??this.labels,t=this.labels;return{count:()=>e.size,...t?{labels:()=>t.readouts().shown?.()??0}:{}}}groupOf(e){return this.signs[0]?.groupOf?.(e)??/* @__PURE__ */ new Set([e])}get size(){return(this.signs[0]??this.labels)?.size??0}};const is=e=>new rs(e),as=`
const PI = 3.141592653589793;
const RE = 6378137.0;
fn anchorS2(a: f32) -> f32 {
  let h = sin(a * 0.5);
  return 2.0 * h * h;
}
// a place → world metres from the view centre (y up), on the globe and flat, by the morph (as a set's placeAt)
fn anchorPlace(rel: vec2f, h: f32) -> vec3f {
  let dl = u.place.x + rel.x;
  let dm = u.place.y + rel.y;
  let c = RE * frame.misc.y;
  let F = vec3f(c * dl, h, -c * dm);
  if (frame.size.w >= 1.0) {
    return F;
  }
  let lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  let dp = lat - frame.misc.x;
  let rr = RE + h;
  let cl = cos(lat);
  let k = anchorS2(dp) + frame.misc.y * cl * anchorS2(dl);
  let G = vec3f(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + frame.geo.y * cl * anchorS2(dl)));
  return mix(G, F, frame.size.w);
}
fn anchorTexel(e: i32, k: i32) -> vec4f {
  let i = e * 2 + k;
  return textureLoad(places, vec2i(i % 4096, i / 4096), 0);
}
fn anchorOf(e: i32) -> vec4f {
  let a = anchorTexel(e, 0);
  if (a.w < 0.5) {
    return vec4f(0.0);
  }
  let b = anchorTexel(e, 1);
  let h = select(a.z, a.z * u.place.z, a.w < 1.5) + b.y;
  return vec4f(anchorPlace(a.xy, h), 1.0);
}
fn anchorTurn(e: i32) -> f32 {
  return anchorTexel(e, 1).x;
}
fn anchorShaped(e: i32) -> bool {
  return false;
}
fn anchorBehind(p: vec3f) -> bool {
  return frame.size.w < 0.5 && dot(p - vec3f(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
`,os=`
fn anchorOf(e: i32) -> vec4f {
  let m = liveOf(e);
  let w = whereAt(m, jOf(e), s.clock.x);
  let j = i32(w.z);
  if ((j < 0 && s.misc.w < 0.5) || m.y < 1.0) {
    return vec4f(0.0);
  }
  return vec4f(placeAt(w.xy, heightOf(w.xy, altNow(e)) + u.place.w), 1.0);
}
fn anchorTurn(e: i32) -> f32 {
  let m = liveOf(e);
  let w = whereAt(m, jOf(e), s.clock.x);
  return headingAt(m, i32(w.z), s.clock.x);
}
fn anchorShaped(e: i32) -> bool {
  return lookOf(e).z > 0.5;
}
fn anchorBehind(p: vec3f) -> bool {
  return behindGlobe(p);
}
`,ss=[`things`,`tcol`,`pcol`,`vcol`,`ground`,`ramp`,`found`],cs=ss.length,ls=/* @__PURE__ */ new Set([`trails`,`pose`,`found`]),us=(e,t=``)=>`${nn}
const TIME16: i32 = ${+(e.time===`u16`)};
const PLACE16: i32 = ${+(e.place===`u16`)};
const VALUE: i32 = ${e.value===`u8`?1:e.value===`f32`?2:0};
const STEP: i32 = ${+!!e.step};
const COLOR: i32 = ${e.color===`ramp`?2:+(e.color===`thing`)};
const SIZE: i32 = ${e.size===`value`?2:+(e.size===`thing`)};
const ROW: i32 = ${e.row};
const GRID: i32 = ${e.grid};
const FOLD: i32 = ${+!!e.fold};
const ALT: i32 = ${e.alt===`u8`?1:e.alt===`f32`?2:0};
const OVER_SEA: i32 = ${e.alt&&e.over===`sea`?1:0};
struct S { clock: vec4f, origin: vec4f, box: vec4f, gridA: vec4f, gridB: vec4f, style: vec4f, color: vec4f, outline: vec4f, trail: vec4f, trailColor: vec4f, value: vec4f, near: vec4f, misc: vec4f, grid: vec4f, sz: vec4f, fx: vec4f, gauge: vec4f, gaugeLo: vec4f, gaugeMid: vec4f, gaugeHi: vec4f, gaugeLine: vec4f, glyph: vec4f, labelText: vec4f, labelHalo: vec4f, shadow: vec4f, shadowLook: vec4f, dotSize: vec4f, shapeSize: vec4f, pc: vec4f, parts: array<vec4f, 32>, link: vec4f, linkColor: vec4f, linkMore: vec4f, air: vec4f, att: vec4f, pivot: vec4f, attv: vec4f }
@group(0) @binding(1) var<uniform> s: S;
@group(1) @binding(0) var things: texture_2d<f32>;
@group(1) @binding(1) var tcol: texture_2d<f32>;
@group(1) @binding(2) var pcol: texture_2d<f32>;
@group(1) @binding(3) var vcol: texture_2d<f32>;
@group(1) @binding(4) var ground: texture_2d<f32>;
@group(1) @binding(5) var ramp: texture_2d<f32>;
@group(1) @binding(6) var found: texture_2d<f32>;
${e.alt&&ls.has(t)?`@group(1) @binding(${cs}) var acol: texture_2d<f32>;`:``}
const PI = 3.141592653589793;
const RE = 6378137.0;
${vn}
`,ds=`
fn s2(a: f32) -> f32 {
  let h = sin(a * 0.5);
  return 2.0 * h * h;
}
// a place → world metres from the view centre (y up), on the globe and flat, by the morph (as the point mark)
fn placeAt(rel: vec2f, h: f32) -> vec3f {
  let dl = s.origin.x + rel.x;
  let dm = s.origin.y + rel.y;
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
fn behindGlobe(p: vec3f) -> bool {
  return frame.size.w < 0.5 && dot(p - vec3f(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
// moved along the line to the eye by half of px: the same place on screen, in front of the ground under it
fn towardEye(p: vec3f, px: f32) -> vec3f {
  let toEye = frame.eye.xyz - p;
  let d = length(toEye);
  let mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(px * frame.size.z * 0.5 * mpp, d * 0.5);
}
// a quad's four corners (two triangles by the index: 0 1 2 · 2 1 3)
fn cornerOf(v: i32) -> vec2f {
  let x = select(-1.0, 1.0, v == 1 || v == 3);
  let y = select(-1.0, 1.0, v >= 2);
  return vec2f(x, y);
}
// a thing's slots: (first slot, samples, slots, the slot of its oldest sample)
fn metaOf(e: i32) -> vec4f {
  let i = e * 2;
  return textureLoad(things, vec2i(i % 4096, i / 4096), 0);
}
// its look: (colour 0xRRGGBB, dot px; 0: the set's)
fn lookOf(e: i32) -> vec4f {
  let i = e * 2 + 1;
  return textureLoad(things, vec2i(i % 4096, i / 4096), 0);
}
// the slot of its j-th sample (0: its oldest; a ring once it holds as many as it keeps); a live group's member
// (m.z = −(slots + 8192 × the group's size)): its samples interleaved with the group's, one row of the ring each
fn slotOf(m: vec4f, j: i32) -> i32 {
  if (m.z < 0.0) {
    let v = i32(-m.z);
    return i32(m.x) + ((i32(m.w) + j) % max(v % 8192, 1)) * (v / 8192);
  }
  let cap = max(i32(m.z), 1);
  return i32(m.x) + (i32(m.w) + j) % cap;
}
fn tAt(sl: i32) -> f32 {
  let col = sl % ROW;
  let row = sl / ROW;
  if (TIME16 == 1) {
    let b = round(textureLoad(tcol, vec2i(col / 2, row), 0) * 255.0);
    let p = select(b.zw, b.xy, col % 2 == 0);
    return (p.x + p.y * 256.0) * s.misc.x;
  }
  return textureLoad(tcol, vec2i(col, row), 0).x;
}
fn relAt(sl: i32) -> vec2f {
  let col = sl % ROW;
  let row = sl / ROW;
  if (PLACE16 == 1) {
    let b = round(textureLoad(pcol, vec2i(col, row), 0) * 255.0);
    let q = vec2f(b.x + b.y * 256.0, b.z + b.w * 256.0) / 65535.0;
    return s.box.xy + q * s.box.zw;
  }
  let f = textureLoad(pcol, vec2i(col / 2, row), 0);
  return select(f.zw, f.xy, col % 2 == 0);
}
fn valueAt(sl: i32) -> f32 {
  let col = sl % ROW;
  let row = sl / ROW;
  if (VALUE == 1) {
    let b = textureLoad(vcol, vec2i(col / 4, row), 0);
    let c = col % 4;
    let v = select(select(select(b.w, b.z, c == 2), b.y, c == 1), b.x, c == 0);
    return s.value.x + v * (s.value.y - s.value.x);
  }
  if (VALUE == 2) {
    return textureLoad(vcol, vec2i(col, row), 0).x;
  }
  return 0.0;
}
// its sample at or before t (-1: before its first): a binary search over its slots, 24 steps at most
fn sampleAt(m: vec4f, t: f32) -> i32 {
  let n = i32(m.y);
  if (n <= 0) {
    return -1;
  }
  if (t < tAt(slotOf(m, 0))) {
    return -1;
  }
  var lo = 0;
  var hi = n - 1;
  if (t >= tAt(slotOf(m, hi))) {
    return hi;
  }
  for (var k = 0; k < 24; k++) {
    if (hi - lo <= 1) {
      break;
    }
    let mid = (lo + hi) / 2;
    if (tAt(slotOf(m, mid)) <= t) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  return lo;
}
// its sample at the time as the found pass wrote it (j + 1 in three bytes; 0: before its first)
fn jOf(e: i32) -> i32 {
  let b = round(textureLoad(found, vec2i(e % 4096, e / 4096), 0) * 255.0);
  return i32(b.x + b.y * 256.0 + b.z * 65536.0) - 1;
}
// its slots as the drawings read them: none while it is folded into another (a relation's window holds it now: the
// found pass's alpha), so every drawing of it (its dot, trail, shape, gauge, link, a card or an icon placed on it) hides
fn liveOf(e: i32) -> vec4f {
  var m = metaOf(e);
  if (FOLD == 1) {
    if (textureLoad(found, vec2i(e % 4096, e / 4096), 0).a < 0.5) {
      m.y = 0.0;
    }
  }
  return m;
}
// where it is at t: its place (xy), its sample before (z; -1 before its first), how far on to the next (w)
fn whereAt(m: vec4f, j: i32, t: f32) -> vec4f {
  let n = i32(m.y);
  let sa = slotOf(m, max(j, 0));
  if (j < 0 || j >= n - 1) {
    let p = relAt(sa);
    return vec4f(p, f32(j), 0.0);
  }
  let sb = slotOf(m, j + 1);
  let ta = tAt(sa);
  let tb = tAt(sb);
  let f = select(0.0, clamp((t - ta) / (tb - ta), 0.0, 1.0), tb > ta);
  return vec4f(mix(relAt(sa), relAt(sb), f), f32(j), f);
}
fn valueBetween(m: vec4f, j: i32, f: f32) -> f32 {
  if (VALUE == 0) {
    return 0.0;
  }
  let n = i32(m.y);
  let a = clamp(j, 0, max(n - 1, 0));
  let b = min(a + 1, max(n - 1, 0));
  // (a step: held from its sample to the next, never through the values between)
  if (STEP == 1) {
    return valueAt(slotOf(m, a));
  }
  return mix(valueAt(slotOf(m, a)), valueAt(slotOf(m, b)), f);
}
// the ground: two GRID × GRID levels in one texture (rows 0…: the whole set's box; GRID…: round the view), metres
fn gLoad(x: i32, y: i32) -> f32 {
  return textureLoad(ground, vec2i(clamp(x, 0, GRID - 1), clamp(y, 0, 2 * GRID - 1)), 0).x;
}
fn gLevel(uv: vec2f, base: i32) -> f32 {
  let g = clamp(uv, vec2f(0.0), vec2f(1.0)) * f32(GRID - 1);
  let i = vec2i(floor(g));
  let f = g - vec2f(i);
  let x1 = min(i.x + 1, GRID - 1);
  let y1 = min(i.y + 1, GRID - 1);
  let a = mix(gLoad(i.x, base + i.y), gLoad(x1, base + i.y), f.x);
  let b = mix(gLoad(i.x, base + y1), gLoad(x1, base + y1), f.x);
  return mix(a, b, f.y);
}
fn groundAt(rel: vec2f) -> f32 {
  let b = (rel - s.gridB.xy) * s.gridB.zw;
  if (s.grid.y > 0.5 && b.x >= 0.0 && b.y >= 0.0 && b.x <= 1.0 && b.y <= 1.0) {
    return gLevel(b, GRID);
  }
  if (s.grid.x > 0.5) {
    return gLevel((rel - s.gridA.xy) * s.gridA.zw, 0);
  }
  return 0.0;
}
fn heightAt(rel: vec2f) -> f32 {
  return groundAt(rel) * s.origin.z + s.origin.w;
}
// its altitude at a slot (m: a byte over its domain, or a 32-bit float; read by the programs that bind its column)
fn altAt(sl: i32) -> f32 {
  /*@altAt*/
  return 0.0;
}
// its altitude between its samples (j the one before, f how far on: as its place)
fn altBetween(m: vec4f, j: i32, f: f32) -> f32 {
  if (ALT == 0) {
    return 0.0;
  }
  let n = i32(m.y);
  let a = clamp(j, 0, max(n - 1, 0));
  let b = min(a + 1, max(n - 1, 0));
  return mix(altAt(slotOf(m, a)), altAt(slotOf(m, b)), f);
}
// how far on from sample j to the next it is at t (0 before its first, after its last)
fn timeShare(m: vec4f, j: i32, t: f32) -> f32 {
  let n = i32(m.y);
  if (j < 0 || j >= n - 1) {
    return 0.0;
  }
  let ta = tAt(slotOf(m, j));
  let tb = tAt(slotOf(m, j + 1));
  return select(0.0, clamp((t - ta) / (tb - ta), 0.0, 1.0), tb > ta);
}
// its altitude now (m), as the found pass wrote it (its second block of rows: 1 cm steps from −500 m); none: 0
fn altNow(e: i32) -> f32 {
  if (ALT == 0) {
    return 0.0;
  }
  let rows = (i32(s.misc.z) + 4095) / 4096;
  let b = round(textureLoad(found, vec2i(e % 4096, rows + e / 4096), 0) * 255.0);
  return (b.x + b.y * 256.0 + b.z * 65536.0) * 0.01 - 500.0;
}
// how far it is over what it stands on (m): its altitude over the ground; over the sea, its height over the ground
// under it (never under it); on the ground, none
fn clearOf(rel: vec2f, alt: f32) -> f32 {
  if (ALT == 0) {
    return 0.0;
  }
  if (OVER_SEA == 1) {
    return max(alt - groundAt(rel), 0.0);
  }
  return max(alt, 0.0);
}
// its height (m): the ground under it (exaggerated) and its lift, and as far over it as it flies
fn heightOf(rel: vec2f, alt: f32) -> f32 {
  return heightAt(rel) + clearOf(rel, alt);
}
fn rampAt(v: f32) -> vec4f {
  let t = clamp((v - s.value.z) / max(s.value.w - s.value.z, 1e-9), 0.0, 1.0);
  return textureLoad(ramp, vec2i(i32(t * 255.0 + 0.5), 0), 0);
}
fn colourOf(look: vec4f, v: f32) -> vec4f {
  if (COLOR == 2) {
    return rampAt(v);
  }
  if (COLOR == 1) {
    return vec4f(unpackRGB(look.x), 1.0);
  }
  return s.color;
}
// a gauge's colour at f (0 empty … 1 full): low → middle → high
fn gaugeColour(f: f32) -> vec3f {
  return select(mix(s.gaugeMid.rgb, s.gaugeHi.rgb, f * 2.0 - 1.0), mix(s.gaugeLo.rgb, s.gaugeMid.rgb, f * 2.0), f < 0.5);
}
// its dot's size (CSS px): one for all, its own, or its value over time (its domain → a range of sizes)
fn sizeOf(look: vec4f, v: f32) -> f32 {
  if (SIZE == 2) {
    let t = clamp((v - s.sz.x) / max(s.sz.y - s.sz.x, 1e-9), 0.0, 1.0);
    return mix(s.sz.z, s.sz.w, t);
  }
  if (SIZE == 1) {
    return look.y;
  }
  return s.style.x;
}
fn angleDiff(a: f32, b: f32) -> f32 {
  let d = b - a;
  return d - 2.0 * PI * floor((d + PI) / (2.0 * PI));
}
// the heading along its k-th piece (radians clockwise from north); where it stands, the piece before (6 back)
fn pieceHeading(m: vec4f, k: i32) -> f32 {
  let n = i32(m.y);
  for (var back = 0; back < 6; back++) {
    let i = k - back;
    if (i >= 0 && i + 1 < n) {
      let d = relAt(slotOf(m, i + 1)) - relAt(slotOf(m, i));
      if (dot(d, d) > 1e-20) {
        return atan2(d.x, d.y);
      }
    }
  }
  return 0.0;
}
// its heading at t: along its piece, turning through each corner over near.w seconds (half before it, half after)
fn headingAt(m: vec4f, j: i32, t: f32) -> f32 {
  let n = i32(m.y);
  let k = clamp(j, 0, max(n - 2, 0));
  let h = pieceHeading(m, k);
  if (s.near.w <= 0.0 || n < 3) {
    return h;
  }
  let ta = tAt(slotOf(m, k));
  let tb = tAt(slotOf(m, k + 1));
  let tau = min(s.near.w, max(tb - ta, 0.001) * 0.5);
  if (k > 0 && t - ta < tau) {
    let h0 = pieceHeading(m, k - 1);
    return h0 + angleDiff(h0, h) * (0.5 + 0.5 * clamp((t - ta) / tau, 0.0, 1.0));
  }
  if (k + 2 < n && tb - t < tau) {
    let h1 = pieceHeading(m, k + 1);
    return h + angleDiff(h, h1) * 0.5 * (1.0 - clamp((tb - t) / tau, 0.0, 1.0));
  }
  return h;
}
`,fs=`
// each thing's link (core › entities › placement: a reference read on the chip): an arc from where it is now (its
// track at the clock, as its dot) to its own place (links: the place, its height m, 1 when it has one), rising to
// link.y m at its middle (0: a quarter of its length); faint (link.z) but for a thing focused (hovered, selected: its
// look's w) at link.w; SEG pieces a link (linkMore.x), drawn as the trails are
@group(1) @binding(${cs}) var links: texture_2d<f32>;
struct O { @builtin(position) pos: vec4f, @location(0) col: vec4f, @location(1) side: f32, @location(2) px: f32 }
// a link's point at u (0 the thing, at its height ha; 1 its place, at hb): between them, risen by the arc
fn linkAt(a: vec2f, b: vec2f, ha: f32, hb: f32, rise: f32, u: f32) -> vec3f {
  let p = mix(a, b, u);
  return towardEye(placeAt(p, mix(ha, hb, u) + rise * 4.0 * u * (1.0 - u)), 1.5);
}
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let SEG = max(i32(s.linkMore.x), 1);
  let e = i32(iid) / SEG;
  let k = i32(iid) - e * SEG;
  let cs = cornerOf(i32(vid));
  let m = liveOf(e);
  let look = lookOf(e);
  let L = textureLoad(links, vec2i(e % 4096, e / 4096), 0);
  let w = whereAt(m, jOf(e), s.clock.x);
  let j = i32(w.z);
  var o: O;
  o.side = cs.y;
  o.px = 1.0;
  o.col = vec4f(0.0);
  o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
  if (L.w < 0.5 || m.y < 1.0 || (j < 0 && s.misc.w < 0.5)) {
    return o;
  }
  let span = length((L.xy - w.xy) * vec2f(RE * frame.misc.y, RE * frame.misc.y));
  let rise = select(min(span * 0.25, 400.0), s.link.y, s.link.y > 0.0);
  let hb = L.z * s.origin.z;
  let ha = heightOf(w.xy, altNow(e));
  let A = linkAt(w.xy, L.xy, ha, hb, rise, f32(k) / f32(SEG));
  let B = linkAt(w.xy, L.xy, ha, hb, rise, f32(k + 1) / f32(SEG));
  var ca = frame.viewProj * vec4f(A, 1.0);
  var cb = frame.viewProj * vec4f(B, 1.0);
  if (ca.w < 0.01 || cb.w < 0.01) {
    return o;
  }
  let halfPx = frame.size.xy * 0.5;
  let sa2 = ca.xy / ca.w * halfPx;
  let sb2 = cb.xy / cb.w * halfPx;
  var dir = sb2 - sa2;
  let len = length(dir);
  dir = select(vec2f(1.0, 0.0), dir / len, len > 1e-6);
  let nrm = vec2f(-dir.y, dir.x);
  let w2 = sizePx(s.link.x, s.linkMore.yzwx * vec4f(1.0, 1.0, 1.0, 0.0), A) * frame.size.z * 0.5 + 0.5;
  let end = cs.x;
  let c = select(ca, cb, end > 0.0);
  let offPx = nrm * cs.y * w2 + dir * end * w2;
  o.pos = clipOut(vec4f(c.xy + offPx / halfPx * c.w, c.z - s.style.w * c.w, c.w));
  let focus = look.w > 0.5;
  o.col = vec4f(s.linkColor.rgb, s.linkColor.a * select(s.link.z, s.link.w, focus) * s.style.z);
  o.px = w2;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let edge = 1.0 - smoothstep(1.0 - 1.0 / max(i.px, 1.0), 1.0, abs(i.side));
  let alpha = i.col.a * edge;
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(i.col.rgb, 0.0), alpha);
}
`,ps=`
@group(1) @binding(${cs}) var glyphs: texture_2d<f32>;
@group(1) @binding(${cs+1}) var smp: sampler;
// the digit of v at a place (0 ones, 1 tens, 2 hundreds)
fn digitOf(v: i32, place: i32) -> i32 {
  let p = select(select(1, 10, place == 1), 100, place == 2);
  return (v / p) % 10;
}
// a glyph's fill and halo at (x, y) from its cell's top left, in label heights
fn glyphAt(code: i32, x: f32, y: f32) -> vec2f {
  if (x < 0.0 || x > s.glyph.w || y < 0.0 || y > 1.0) {
    return vec2f(0.0, 0.0);
  }
  let uv = vec2f((f32(code) * s.glyph.w + x) / (11.0 * s.glyph.w), y);
  return textureSampleLevel(glyphs, smp, uv, 0.0).rg;
}
// the label at q (device px in the bar of px): its fill and halo; the glyphs either side are read too (a halo spills)
fn labelAt(q: vec2f, px: vec2f, pct: i32) -> vec2f {
  let h = s.labelText.w * frame.size.z;
  let n = select(select(2, 3, pct >= 10), 4, pct >= 100);
  let w = (f32(n - 1) * s.glyph.x + s.glyph.y) * h;
  let x = (q.x - (px.x - w) * 0.5) / h;
  let y = ((px.y + h) * 0.5 - q.y) / h;
  let k = clamp(i32(floor(x / s.glyph.x)), 0, n - 1);
  var out = vec2f(0.0, 0.0);
  for (var j = max(k - 1, 0); j <= min(k + 1, n - 1); j++) {
    let code = select(digitOf(pct, n - 2 - j), 10, j == n - 1);
    out = max(out, glyphAt(code, x - f32(j) * s.glyph.x + s.glyph.z, y));
  }
  return out;
}
`,ms=e=>`${e.label?ps:``}
struct O { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) fill: f32, @location(2) col: vec3f, @location(3) px: vec2f, @location(4) @interpolate(flat) id: f32, @location(5) @interpolate(flat) pct: f32 }
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let e = i32(iid);
  let m = liveOf(e);
  let w = whereAt(m, jOf(e), s.clock.x);
  let j = i32(w.z);
  let f = clamp((valueBetween(m, j, w.w) - s.fx.z) / max(s.fx.w - s.fx.z, 1e-9), 0.0, 1.0);
  let corner = cornerOf(i32(vid));
  let p0 = placeAt(w.xy, heightOf(w.xy, altNow(e)) - s.origin.w + s.gauge.z);
  let d = length(frame.eye.xyz - p0);
  let hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0 || d > s.gauge.w || behindGlobe(p0);
  // (drawn over what stands within fx.y metres in front of it: moved that far towards the eye, the same pixels)
  let p = p0 + (frame.eye.xyz - p0) / max(d, 0.001) * min(s.fx.y, d * 0.5);
  let clip = frame.viewProj * vec4f(p, 1.0);
  let off = corner * s.gauge.xy * frame.size.z / frame.size.xy * clip.w;
  var o: O;
  o.pos = select(clipOut(vec4f(clip.xy + off, clip.z - s.style.w * clip.w, clip.w)), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.uv = corner * 0.5 + 0.5;
  o.fill = f;
  o.col = gaugeColour(f);
  o.px = s.gauge.xy * frame.size.z;
  o.id = f32(e);
  o.pct = floor(f * 100.0 + 0.5);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let q = i.uv * i.px;
  let edge = min(min(q.x, i.px.x - q.x), min(q.y, i.px.y - q.y));
  let line = edge < frame.size.z;
  let full = i.uv.x <= i.fill;
  var rgb = select(select(vec3f(0.08, 0.1, 0.12), i.col, full), s.gaugeLine.rgb, line);
  var alpha = select(select(0.6, 1.0, full), s.gaugeLine.a, line) * s.style.z;${e.label?`
  let t = labelAt(q, i.px, i32(i.pct));
  rgb = mix(mix(rgb, s.labelHalo.rgb, t.y * s.labelHalo.a), s.labelText.rgb, t.x * s.labelText.a);
  alpha = max(alpha, max(t.x * s.labelText.a, t.y * s.labelHalo.a) * s.style.z);`:``}
  if (frame.pick.x > 0.5) {
    return pickOut(s.misc.y, i.id);
  }
  return vec4f(themed(rgb, 0.0), alpha);
}
`,hs=e=>`
@group(1) @binding(${cs+ +!!e.alt}) var fold: texture_2d<f32>;
fn foldAt(k: i32) -> vec4f {
  return textureLoad(fold, vec2i(k % 4096, k / 4096), 0);
}
fn foldedAt(e: i32, t: f32) -> bool {
  let ix = foldAt(e);
  let n = i32(ix.y);
  let a = i32(ix.x);
  if (n <= 0 || foldAt(a).x > t) {
    return false;
  }
  var lo = 0;
  var hi = n - 1;
  for (var k = 0; k < 24; k++) {
    if (hi <= lo) {
      break;
    }
    let mid = (lo + hi + 1) / 2;
    if (foldAt(a + mid).x <= t) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return t < foldAt(a + lo).y;
}
`,gs=e=>`${e.fold?hs(e):``}
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let q = vec2i(pos.xy);
  // (in the air: below its rows of samples, a row of heights: each thing's altitude now, 1 cm steps from −500 m)
  let rows = (i32(s.misc.z) + 4095) / 4096;
  if (ALT > 0 && q.y >= rows) {
    let ea = (q.y - rows) * 4096 + q.x;
    if (ea >= i32(s.misc.z)) {
      return vec4f(0.0);
    }
    let ma = metaOf(ea);
    let ja = sampleAt(ma, s.clock.x);
    let u = clamp(round((altBetween(ma, ja, timeShare(ma, ja, s.clock.x)) + 500.0) * 100.0), 0.0, 16777215.0);
    let hi = floor(u / 65536.0);
    let mid = floor((u - hi * 65536.0) / 256.0);
    return vec4f(u - hi * 65536.0 - mid * 256.0, mid, hi, 255.0) / 255.0;
  }
  let e = q.y * 4096 + q.x;
  if (e >= i32(s.misc.z)) {
    return vec4f(0.0);
  }
  let j = sampleAt(metaOf(e), s.clock.x) + 1;
  let folded = ${e.fold?`foldedAt(e, s.clock.x)`:`false`};
  return vec4f(f32(j % 256), f32((j / 256) % 256), f32(j / 65536), select(255.0, 0.0, folded)) / 255.0;
}
`,_s=e=>{let t=e.pose,n=e=>e===`climb`||e===`turn`?1:e===`u8`?2:e===`f32`?3:0,r=cs+ +!!e.alt,i=r+ +(t.pitch===`u8`||t.pitch===`f32`);return`
const PITCH: i32 = ${n(t.pitch)};
const BANK: i32 = ${n(t.bank)};
${t.pitch===`u8`||t.pitch===`f32`?`@group(1) @binding(${r}) var pitchCol: texture_2d<f32>;`:``}
${t.bank===`u8`||t.bank===`f32`?`@group(1) @binding(${i}) var bankCol: texture_2d<f32>;`:``}
// a column's value at a slot (a byte over its domain lo … hi, or a 32-bit float)
fn columnAt(b: vec4f, col: i32, isByte: bool, lo: f32, hi: f32) -> f32 {
  if (!isByte) {
    return b.x;
  }
  let c = col % 4;
  let v = select(select(select(b.w, b.z, c == 2), b.y, c == 1), b.x, c == 0);
  return lo + v * (hi - lo);
}
fn pitchAt(sl: i32) -> f32 {
  ${t.pitch===`u8`?`return columnAt(textureLoad(pitchCol, vec2i((sl % ROW) / 4, sl / ROW), 0), sl % ROW, true, s.attv.x, s.attv.y);`:t.pitch===`f32`?`return columnAt(textureLoad(pitchCol, vec2i(sl % ROW, sl / ROW), 0), 0, false, 0.0, 0.0);`:`return 0.0;`}
}
fn bankAt(sl: i32) -> f32 {
  ${t.bank===`u8`?`return columnAt(textureLoad(bankCol, vec2i((sl % ROW) / 4, sl / ROW), 0), sl % ROW, true, s.attv.z, s.attv.w);`:t.bank===`f32`?`return columnAt(textureLoad(bankCol, vec2i(sl % ROW, sl / ROW), 0), 0, false, 0.0, 0.0);`:`return 0.0;`}
}
// the metres a unit of the places is where it is (its latitude's)
fn metresAt(rel: vec2f) -> f32 {
  let lat = 2.0 * atan(exp(frame.geo.x + s.origin.y + rel.y)) - PI * 0.5;
  return RE * cos(lat);
}
// the share of a corner's turn its heading has made at time x (the corner at tk, its turn eased over tl before it and
// tr after it: as headingAt turns it)
fn turnShare(x: f32, tk: f32, tl: f32, tr: f32) -> f32 {
  if (x < tk) {
    return 0.5 * clamp(1.0 - (tk - x) / max(tl, 0.001), 0.0, 1.0);
  }
  return 0.5 + 0.5 * clamp((x - tk) / max(tr, 0.001), 0.0, 1.0);
}
// its pitch from its climb and its bank from its turn at t (radians), over its path from t − att.x to t + att.x: the
// height it gains over the metres it goes; the turn its heading makes over that time at its speed there
fn pathPose(m: vec4f, j: i32, t: f32) -> vec2f {
  let n = i32(m.y);
  if (n < 2) {
    return vec2f(0.0, 0.0);
  }
  let D = max(s.att.x, 0.05);
  let tw = select(s.near.w, D, s.near.w <= 0.0);
  let t0 = max(t - D, tAt(slotOf(m, 0)));
  let t1 = min(t + D, tAt(slotOf(m, n - 1)));
  // the pieces round it: back to one that starts before t0 − tw, on to one that ends after t1 + tw (24 each way at most)
  var k0 = clamp(j, 0, n - 2);
  for (var i = 0; i < 24; i++) {
    if (k0 <= 0 || tAt(slotOf(m, k0)) <= t0 - tw) {
      break;
    }
    k0 = k0 - 1;
  }
  var k1 = clamp(j, 0, n - 2);
  for (var i = 0; i < 24; i++) {
    if (k1 >= n - 2 || tAt(slotOf(m, k1 + 1)) >= t1 + tw) {
      break;
    }
    k1 = k1 + 1;
  }
  var len = 0.0;
  var turn = 0.0;
  var a0 = altAt(slotOf(m, k0));
  var a1 = a0;
  var tp = tAt(slotOf(m, max(k0 - 1, 0)));
  for (var k = k0; k <= k1; k++) {
    let sa = slotOf(m, k);
    let sb = slotOf(m, k + 1);
    let ta = tAt(sa);
    let tb = tAt(sb);
    let dt = max(tb - ta, 0.0001);
    let pa = relAt(sa);
    // its metres within the window; its height at the window's ends
    let inside = max(min(tb, t1) - max(ta, t0), 0.0);
    len = len + length(relAt(sb) - pa) * metresAt(pa) * inside / dt;
    if (t0 >= ta && t0 <= tb) {
      a0 = mix(altAt(sa), altAt(sb), clamp((t0 - ta) / dt, 0.0, 1.0));
    }
    if (t1 >= ta && t1 <= tb) {
      a1 = mix(altAt(sa), altAt(sb), clamp((t1 - ta) / dt, 0.0, 1.0));
    }
    // the turn at the corner it starts at, as much of it as its heading makes within the window
    if (k > 0) {
      let d = angleDiff(pieceHeading(m, k - 1), pieceHeading(m, k));
      let tl = min(tw, max(ta - tp, 0.001) * 0.5);
      let tr = min(tw, dt * 0.5);
      turn = turn + d * (turnShare(t1, ta, tl, tr) - turnShare(t0, ta, tl, tr));
    }
    tp = ta;
  }
  let span = max(t1 - t0, 0.001);
  return vec2f(atan2(a1 - a0, max(len, 0.001)), atan(((len / span) * (turn / span)) / 9.80665));
}
// its pose at t: from its path (each within its most; a thing with an altitude banks only off the ground: fully once
// it is att.w m over it) or the log's columns (degrees, between its samples); up: how far over the ground it is
fn poseAt(m: vec4f, j: i32, f: f32, t: f32, up: f32) -> vec2f {
  var pb = vec2f(0.0, 0.0);
  if (PITCH == 1 || BANK == 1) {
    pb = pathPose(m, j, t);
  }
  var pitch = select(0.0, clamp(pb.x, -s.att.y, s.att.y), PITCH == 1);
  let off = select(1.0, smoothstep(0.0, 1.0, up / max(s.att.w, 0.001)), ALT > 0);
  var bank = select(0.0, clamp(pb.y, -s.att.z, s.att.z) * off, BANK == 1);
  let n = i32(m.y);
  let a = slotOf(m, clamp(j, 0, max(n - 1, 0)));
  let b = slotOf(m, min(clamp(j, 0, max(n - 1, 0)) + 1, max(n - 1, 0)));
  if (PITCH >= 2) {
    pitch = mix(pitchAt(a), pitchAt(b), f) * 0.017453293;
  }
  if (BANK >= 2) {
    bank = mix(bankAt(a), bankAt(b), f) * 0.017453293;
  }
  return vec2f(pitch, bank);
}
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let q = vec2i(pos.xy);
  let e = q.y * 4096 + q.x;
  // (only the things drawn as shapes now)
  if (e >= i32(s.misc.z) || lookOf(e).z < 0.5) {
    return vec4f(0.0, 128.0, 0.0, 128.0) / 255.0;
  }
  let m = liveOf(e);
  let w = whereAt(m, jOf(e), s.clock.x);
  let pb = poseAt(m, i32(w.z), w.w, s.clock.x, clearOf(w.xy, altNow(e)));
  let u = round(clamp(pb / PI + vec2f(0.5), vec2f(0.0), vec2f(1.0)) * 65535.0);
  let hi = floor(u / 256.0);
  return vec4f(u.x - hi.x * 256.0, hi.x, u.y - hi.y * 256.0, hi.y) / 255.0;
}
`},vs=e=>{let t=cs+ +(e.joint===`f32`);return`
/*@jointColumnBinding*/
/*@poseBinding*/
// its pose as the pose pass found it (pitch, bank: radians; 16 bits each): none, level
fn poseOf(e: i32) -> vec2f {
  /*@poseOf*/
  return vec2f(0.0, 0.0);
}
// p turned by its pose: its bank round its length (its right side down in a right turn), then its pitch round its width
// (its nose up)
fn attitude(p: vec3f, pb: vec2f) -> vec3f {
  let cb = cos(pb.y);
  let sb = sin(pb.y);
  let b = vec3f(p.x * cb - p.y * sb, p.x * sb + p.y * cb, p.z);
  let cp = cos(pb.x);
  let sp = sin(pb.x);
  return vec3f(b.x, b.y * cp + b.z * sp, b.z * cp - b.y * sp);
}
// the log value a joint reads (its column: a 32-bit float a slot), at a slot
fn jointColumnAt(sl: i32) -> f32 {
  /*@jointColumnAt*/
  return 0.0;
}
// that value at the time: between the samples either side, as its place (core › entities › drive › data)
fn jointColumn(m: vec4f, j: i32, f: f32) -> f32 {
  let n = i32(m.y);
  let a = clamp(j, 0, max(n - 1, 0));
  let b = min(a + 1, max(n - 1, 0));
  return mix(jointColumnAt(slotOf(m, a)), jointColumnAt(slotOf(m, b)), select(f, 0.0, j < 0));
}
// core › entities › kinematics (the shapes' parts): p turned by t radians round the unit axis a (Rodrigues)
fn rotAxis(p: vec3f, a: vec3f, t: f32) -> vec3f {
  let c = cos(t);
  return p * c + cross(a, p) * sin(t) + a * dot(a, p) * (1.0 - c);
}
// your formulas (a joint's value of the clock and the thing's seed; degrees, or metres for a slider)
fn jointFormula(k: i32, time: f32, seed: f32) -> f32 {
  /*@jointFormula*/
  return 0.0;
}
// a joint's value now (radians; a slider's metres): the set's rate from its clock (each thing's phase by its seed, an
// offset a change of rate keeps), your formula, or its log value × its factor (a wheel: metres it rolled, turned by
// its radius; a hinge: degrees; a slider: metres); within its limits (parts: each joint 4 texels, as an instance set's)
fn jointValue(k: i32, seed: f32, m: vec4f, j: i32, f: f32) -> f32 {
  let d = s.parts[k * 4 + 2];
  let kind = s.parts[k * 4].w;
  var v = 0.0;
  if (d.x == 1.0) {
    v = seed * 6.2831853 + d.y * (frame.eye.w - s.pc.y) + s.parts[k * 4 + 3].z;
  } else if (d.x == 2.0) {
    v = jointFormula(k, frame.eye.w, seed) * select(0.017453293, 1.0, kind == 3.0);
  } else if (d.x == 3.0) {
    let u = jointColumn(m, j, f) * d.y;
    v = select(select(u * 0.017453293, u / max(s.parts[k * 4 + 1].w, 0.001), kind == 2.0), u, kind == 3.0);
  }
  if (d.z < d.w) {
    v = clamp(v, d.z, d.w);
  }
  return v;
}
// a vertex (q, its normal n) of part \`part\` moved by that part's joints
struct Moved { q: vec3f, n: vec3f }
fn jointMove(q: vec3f, n: vec3f, part: i32, seed: f32, m: vec4f, j: i32, f: f32) -> Moved {
  var mv = Moved(q, n);
  if (part == 0) {
    return mv;
  }
  for (var k = 0; k < i32(s.pc.x); k++) {
    if (i32(s.parts[k * 4 + 3].x + 0.5) != part) {
      continue;
    }
    let pv = s.parts[k * 4];
    let ax = s.parts[k * 4 + 1].xyz;
    let v = jointValue(k, seed, m, j, f);
    if (pv.w == 3.0) {
      mv.q = mv.q + ax * v;
    } else {
      mv.q = pv.xyz + rotAxis(mv.q - pv.xyz, ax, v);
      mv.n = rotAxis(mv.n, ax, v);
    }
  }
  return mv;
}
struct I { @builtin(vertex_index) vid: u32, @location(0) ent: f32, @location(3) v: vec4f, @location(4) vc: f32, @location(5) card: f32 }
struct O { @builtin(position) pos: vec4f, @location(0) col: vec3f, @location(1) n: vec3f, @location(2) @interpolate(flat) id: f32 }
@vertex fn vs(i: I) -> O {
  let e = i32(i.ent + 0.5);
  let m = liveOf(e);
  let look = lookOf(e);
  let t = s.clock.x;
  let w = whereAt(m, jOf(e), t);
  let j = i32(w.z);
  let p = placeAt(w.xy, heightOf(w.xy, altNow(e)) - s.origin.w);
  let d = length(frame.eye.xyz - p);
  let a = headingAt(m, j, t) + PI;
  let c = cos(a);
  let sn = sin(a);
  let keep = select(1.0, sizeKeep(s.shapeSize.x * s.near.y, vec4f(1.0, s.shapeSize.y, s.shapeSize.z, 0.0), p), s.shapeSize.w > 0.5);
  // its code: its material, its own colour (256), its piece (part + 16 × group) × 512; its part moved by its joints
  let ci = select(i32(i.card + 0.5), 0, i.card >= 65536.0);
  let mv = jointMove(i.v.xyz, unpackNormal(i.v.w), (ci / 512) % 16, fract(f32(e) * 0.6180339887), m, j, w.w);
  var q = mv.q * s.near.y * keep;
  var n = mv.n;
  // (in the air: turned by its pose about its pivot)
  let pb = poseOf(e);
  if (pb.x != 0.0 || pb.y != 0.0) {
    let pv = s.pivot.xyz * s.near.y * keep;
    q = pv + attitude(q - pv, pb);
    n = attitude(n, pb);
  }
  let r = vec3f(q.x * c - q.z * sn, q.y, q.x * sn + q.z * c);
  let hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0;
  var o: O;
  o.pos = select(clipOut(frame.viewProj * vec4f(p + r, 1.0)), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.n = vec3f(n.x * c - n.z * sn, n.y, n.x * sn + n.z * c);
  let own = i.card < 65536.0 && (ci % 512) >= 256;
  let col = colourOf(look, valueBetween(m, j, w.w));
  o.col = select(col.rgb, vec3f(1.0), own) * unpackRGB(i.vc);
  o.id = f32(e);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let n = normalize(i.n);
  let lit = frame.sunColor.a > 0.5;
  let L = select(normalize(vec3f(0.4, 0.8, 0.45)), normalize(frame.sun.xyz), lit);
  let diff = max(dot(n, L), 0.0);
  let rgb = select(i.col * (0.55 + 0.45 * diff), i.col * (frame.ambient.rgb + frame.sunColor.rgb * diff), lit);
  if (frame.pick.x > 0.5) {
    return pickOut(s.misc.y, i.id);
  }
  return vec4f(themed(rgb, 0.0), 1.0);
}
`.replace(`/*@jointFormula*/`,e.formula?.wgsl??``).replace(`/*@jointColumnBinding*/`,e.joint===`f32`?`@group(1) @binding(${cs}) var jcol: texture_2d<f32>;`:``).replace(`/*@jointColumnAt*/`,e.joint===`f32`?`return textureLoad(jcol, vec2i(sl % ROW, sl / ROW), 0).x;`:``).replace(`/*@poseBinding*/`,e.pose?`@group(1) @binding(${t}) var pose: texture_2d<f32>;`:``).replace(`/*@poseOf*/`,e.pose?`let b = round(textureLoad(pose, vec2i(e % 4096, e / 4096), 0) * 255.0);
  return (vec2f(b.x + b.y * 256.0, b.z + b.w * 256.0) / 65535.0 - vec2f(0.5)) * PI;`:``)};function ys(e){return us(e)+ds}function bs(e,t){let n=t.alt&&ls.has(e)?ds.replace(`/*@altAt*/`,xs):ds;return us(t,e)+n+(e===`found`?gs(t):e===`pose`?_s(t):e===`dots`?`
struct O { @builtin(position) pos: vec4f, @location(0) corner: vec2f, @location(1) col: vec4f, @location(2) @interpolate(flat) id: f32, @location(3) size: f32 }
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let e = i32(iid);
  let m = liveOf(e);
  let look = lookOf(e);
  let w = whereAt(m, jOf(e), s.clock.x);
  let j = i32(w.z);
  let v = valueBetween(m, j, w.w);
  let corner = cornerOf(i32(vid));
  let p = placeAt(w.xy, heightOf(w.xy, altNow(e)));
  let size = sizePx(sizeOf(look, v), s.dotSize, p);
  let on = select(0.0, 1.0, j >= 0 || s.misc.w > 0.5);
  let d = length(frame.eye.xyz - p);
  // drawn as a shape (its look's flag, set when the shapes are chosen): its dot gives way
  let give = select(1.0, 0.0, look.z > 0.5);
  let c = colourOf(look, v);
  let lifted = towardEye(p, size);
  let clip = frame.viewProj * vec4f(lifted, 1.0);
  let off = corner * size * frame.size.z / frame.size.xy * clip.w;
  let hide = on < 0.5 || give <= 0.0 || m.y < 1.0 || behindGlobe(p);
  var o: O;
  o.pos = select(clipOut(vec4f(clip.xy + off, clip.z - s.style.w * clip.w, clip.w)), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.corner = corner;
  o.col = vec4f(c.rgb, c.a * give);
  o.id = f32(e);
  o.size = size * frame.size.z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let r = length(i.corner);
  let px = 2.0 / max(i.size, 1.0);
  let disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
  let ring = select(0.0, smoothstep(1.0 - px * (s.style.y + 1.0), 1.0 - px * s.style.y, r), s.style.y > 0.0);
  let rgb = mix(i.col.rgb, s.outline.rgb, ring * s.outline.a);
  let alpha = i.col.a * disc * s.style.z;
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(s.misc.y, i.id);
  }
  return vec4f(themed(rgb, 0.0), alpha);
}
`:e===`trails`?`
struct O { @builtin(position) pos: vec4f, @location(0) col: vec4f, @location(1) side: f32, @location(2) px: f32 }
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let K = max(i32(s.clock.z), 1);
  let e = i32(iid) / K;
  let seg = i32(iid) - e * K;
  let cs = cornerOf(i32(vid));
  let m = liveOf(e);
  let look = lookOf(e);
  let n = i32(m.y);
  let t = s.clock.x;
  let tt = t - s.clock.y;
  let j = jOf(e);
  var o: O;
  o.side = cs.y;
  o.px = 1.0;
  o.col = vec4f(0.0);
  o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
  // nothing here: before its first sample, older than its first, or older than the window (most pieces, most frames)
  if (j < 0 || j - seg < 0 || m.y < 1.0) {
    return o;
  }
  if (seg > 0 && tAt(slotOf(m, j - seg + 1)) <= tt) {
    return o;
  }
  let ia = max(j - seg, 0);
  let sa = slotOf(m, ia);
  var ta = tAt(sa);
  var pa = relAt(sa);
  var va = valueAt(sa);
  var aa = altAt(sa);
  var pb = pa;
  var tb = ta;
  var vb = va;
  var ab = aa;
  if (seg == 0) {
    if (j < n - 1) {
      let sn = slotOf(m, j + 1);
      let t1 = tAt(sn);
      let f = select(0.0, clamp((t - ta) / (t1 - ta), 0.0, 1.0), t1 > ta);
      pb = mix(pa, relAt(sn), f);
      tb = mix(ta, t1, f);
      vb = mix(va, valueAt(sn), f);
      ab = mix(aa, altAt(sn), f);
    }
  } else {
    let sb = slotOf(m, ia + 1);
    pb = relAt(sb);
    tb = tAt(sb);
    vb = valueAt(sb);
    ab = altAt(sb);
  }
  // (a step: the piece's value its first sample's, to the next)
  if (STEP == 1) {
    vb = va;
  }
  var ok = j >= 0 && j - seg >= 0 && tb > tt && (distance(pa, pb) > 1e-12 || abs(ab - aa) > 0.01);
  if (ta < tt && tb > ta) {
    let f = (tt - ta) / (tb - ta);
    pa = mix(pa, pb, f);
    va = mix(va, vb, f);
    aa = mix(aa, ab, f);
    ta = tt;
  }
  let end = cs.x;
  let side = cs.y;
  let A = towardEye(placeAt(pa, heightOf(pa, aa)), s.trail.z);
  let B = towardEye(placeAt(pb, heightOf(pb, ab)), s.trail.z);
  var ca = frame.viewProj * vec4f(A, 1.0);
  var cb = frame.viewProj * vec4f(B, 1.0);
  let nearW = 0.01;
  if (ca.w < nearW && cb.w < nearW) {
    ok = false;
  }
  if (ca.w < nearW) {
    ca = mix(ca, cb, (nearW - ca.w) / (cb.w - ca.w));
  }
  if (cb.w < nearW) {
    cb = mix(cb, ca, (nearW - cb.w) / (ca.w - cb.w));
  }
  let halfPx = frame.size.xy * 0.5;
  let sa2 = ca.xy / ca.w * halfPx;
  let sb2 = cb.xy / cb.w * halfPx;
  var dir = sb2 - sa2;
  let len = length(dir);
  dir = select(vec2f(1.0, 0.0), dir / len, len > 1e-6);
  let nrm = vec2f(-dir.y, dir.x);
  let w2 = s.trail.x * frame.size.z * 0.5 + 0.5;
  let c = select(ca, cb, end > 0.0);
  let offPx = nrm * side * w2 + dir * end * w2;
  let clip = vec4f(c.xy + offPx / halfPx * c.w, c.z - s.style.w * c.w, c.w);
  let age = clamp((select(ta, tb, end > 0.0) - tt) / max(s.clock.y, 0.001), 0.0, 1.0);
  let col = select(colourOf(look, select(va, vb, end > 0.0)), s.trailColor, s.trail.w > 0.5);
  let glow = s.clock.w * smoothstep(0.85, 1.0, age);
  let hide = !ok || m.y < 1.0 || behindGlobe(select(A, B, end > 0.0));
  o.pos = select(clipOut(clip), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.col = vec4f(mix(col.rgb, vec3f(1.0), glow * 0.5), col.a * s.trail.y * pow(age, s.fx.x));
  o.side = side;
  o.px = w2;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let edge = 1.0 - smoothstep(1.0 - 1.0 / max(i.px, 1.0), 1.0, abs(i.side));
  let alpha = i.col.a * edge;
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(i.col.rgb, 0.0), alpha);
}
`:e===`gauge`?ms(t):e===`shadow`?`
struct I { @builtin(vertex_index) vid: u32, @location(0) ent: f32 }
struct O { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) fade: f32 }
// how much of its shadow is left at a height over the ground (m): shrunk to half and faded away by shadowLook.z m (0:
// never), none past shadowLook.w m (0: no such height); (its share across, its opacity)
fn shadowAt(up: f32) -> vec2f {
  if (s.shadowLook.w > 0.0 && up > s.shadowLook.w) {
    return vec2f(0.0, 0.0);
  }
  let u = select(0.0, clamp(up / max(s.shadowLook.z, 0.001), 0.0, 1.0), s.shadowLook.z > 0.0);
  return vec2f(1.0 - 0.5 * u, (1.0 - u) * (1.0 - u));
}
@vertex fn vs(i: I) -> O {
  let e = i32(i.ent + 0.5);
  let m = liveOf(e);
  let t = s.clock.x;
  let w = whereAt(m, jOf(e), t);
  let j = i32(w.z);
  // on the ground under it, however high it flies
  let p = placeAt(w.xy, heightAt(w.xy) - s.origin.w + 0.03);
  let left = shadowAt(clearOf(w.xy, altNow(e)));
  let d = length(frame.eye.xyz - p);
  let a = headingAt(m, j, t) + PI;
  let c = cos(a);
  let sn = sin(a);
  let uv = cornerOf(i32(i.vid));
  let keep = select(1.0, sizeKeep(s.shapeSize.x * s.near.y, vec4f(1.0, s.shapeSize.y, s.shapeSize.z, 0.0), p), s.shapeSize.w > 0.5);
  let q = vec2f(s.shadow.z + uv.x * s.shadow.x, s.shadow.w + uv.y * s.shadow.y) * s.near.y * keep * left.x;
  let r = vec3f(q.x * c - q.y * sn, 0.0, q.x * sn + q.y * c);
  let hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0 || left.y <= 0.002;
  // (moved towards the eye by 20 px' worth: the same place on screen, over the ground it lies on, whatever it is)
  let clip = frame.viewProj * vec4f(towardEye(p + r, 20.0), 1.0);
  var o: O;
  o.pos = select(clipOut(clip), vec4f(2.0, 2.0, 2.0, 1.0), hide);
  o.uv = uv;
  o.fade = left.y;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let k = s.shadowLook.y;
  let q = max(abs(i.uv) - vec2f(k), vec2f(0.0)) / max(1.0 - k, 0.001);
  let f = 1.0 - smoothstep(0.0, 1.0, length(q));
  let alpha = s.shadowLook.x * f * f * s.style.z * i.fade;
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(0.0, 0.0, 0.0, alpha);
}
`:e===`links`?fs:vs(t))}const xs=`let col = sl % ROW;
  let row = sl / ROW;
  if (ALT == 1) {
    let b = textureLoad(acol, vec2i(col / 4, row), 0);
    let c = col % 4;
    return s.air.x + select(select(select(b.w, b.z, c == 2), b.y, c == 1), b.x, c == 0) * (s.air.y - s.air.x);
  }
  if (ALT == 2) {
    return textureLoad(acol, vec2i(col, row), 0).x;
  }`;function Ss(e){let t=e?ss.length:1;return`${e?ys(e):nn}
struct U { place: vec4f, size: vec4f, look: vec4f, misc: vec4f }
@group(0) @binding(${e?2:1}) var<uniform> u: U;
${e?``:`@group(1) @binding(0) var places: texture_2d<f32>;`}
@group(1) @binding(${t}) var items: texture_2d<f32>;
@group(1) @binding(${t+1}) var atlas: texture_2d<f32>;
@group(1) @binding(${t+2}) var smp: sampler;
${e?os:as}
struct O { @builtin(position) pos: vec4f, @location(0) uv: vec2f, @location(1) @interpolate(flat) cell: vec4f, @location(2) @interpolate(flat) tint: vec4f, @location(3) @interpolate(flat) id: f32 }
fn iconCorner(v: i32) -> vec2f {
  let x = select(-1.0, 1.0, v == 1 || v == 3);
  let y = select(-1.0, 1.0, v >= 2);
  return vec2f(x, y);
}
fn iconItem(e: i32, k: i32) -> vec4f {
  let i = e * 2 + k;
  return textureLoad(items, vec2i(i % 4096, i / 4096), 0);
}
// the screen's angle of a heading at p (radians: 0 points up the screen)
fn iconAngle(p: vec3f, h: f32) -> f32 {
  let a = frame.viewProj * vec4f(p, 1.0);
  let b = frame.viewProj * vec4f(p + vec3f(sin(h), 0.0, -cos(h)), 1.0);
  let d = (b.xy / max(b.w, 0.001) - a.xy / max(a.w, 0.001)) * frame.size.xy;
  return select(0.0, atan2(d.x, d.y), dot(d, d) > 1e-12);
}
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let e = i32(iid);
  let a = anchorOf(e);
  let it = iconItem(e, 0);
  let corner = iconCorner(i32(vid));
  var o: O;
  o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
  o.uv = vec2f(0.0);
  o.cell = vec4f(0.0);
  o.tint = vec4f(0.0);
  o.id = f32(e);
  if (a.w < 0.5 || it.x < 0.0 || anchorShaped(e) || anchorBehind(a.xyz)) {
    return o;
  }
  let size = sizePx(it.y, u.size, a.xyz);
  let ang = select(0.0, iconAngle(a.xyz, anchorTurn(e)), u.look.w > 0.5);
  let c = cos(ang);
  let sn = sin(ang);
  let r = vec2f(corner.x * c + corner.y * sn, -corner.x * sn + corner.y * c);
  let clip = frame.viewProj * vec4f(a.xyz, 1.0);
  if (clip.w < 0.01) {
    return o;
  }
  let off = r * size * frame.size.z / frame.size.xy * clip.w;
  o.pos = clipOut(vec4f(clip.xy + off, clip.z, clip.w));
  o.uv = vec2f(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
  let col = it.x % u.look.y;
  let row = floor(it.x / u.look.y);
  o.cell = vec4f(col / u.look.y, row / u.look.z, 1.0 / u.look.y, 1.0 / u.look.z);
  o.tint = iconItem(e, 1);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  // four taps across the pixel (the atlas has no smaller levels: an icon drawn small stays smooth)
  let q = clamp(i.uv, vec2f(0.0), vec2f(1.0));
  let dx = dpdx(q) * 0.25;
  let dy = dpdy(q) * 0.25;
  let at = i.cell.xy + q * i.cell.zw;
  let k = i.cell.zw;
  let c = (textureSampleLevel(atlas, smp, at + (dx + dy) * k, 0.0) + textureSampleLevel(atlas, smp, at + (dx - dy) * k, 0.0) + textureSampleLevel(atlas, smp, at - (dx - dy) * k, 0.0) + textureSampleLevel(atlas, smp, at - (dx + dy) * k, 0.0)) * 0.25;
  let alpha = c.a * i.tint.a * u.look.x;
  if (alpha <= 0.004) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.misc.x, i.id);
  }
  // (the atlas holds colours times alpha)
  return vec4f(themed(c.rgb / max(c.a, 0.0001) * i.tint.rgb, 0.0), alpha);
}
`}const Cs=`
const float PI = 3.141592653589793;
const float RE = 6378137.0;
float anchorS2(float a) {
  float h = sin(a * 0.5);
  return 2.0 * h * h;
}
vec3 anchorPlace(vec2 rel, float h) {
  float dl = u.place.x + rel.x;
  float dm = u.place.y + rel.y;
  float c = RE * frame.misc.y;
  vec3 F = vec3(c * dl, h, -c * dm);
  if (frame.size.w >= 1.0) {
    return F;
  }
  float lat = 2.0 * atan(exp(frame.geo.x + dm)) - PI * 0.5;
  float dp = lat - frame.misc.x;
  float rr = RE + h;
  float cl = cos(lat);
  float k = anchorS2(dp) + frame.misc.y * cl * anchorS2(dl);
  vec3 G = vec3(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + frame.geo.y * cl * anchorS2(dl)));
  return mix(G, F, frame.size.w);
}
vec4 anchorTexel(int e, int k) {
  int i = e * 2 + k;
  return texelFetch(places, ivec2(i % 4096, i / 4096), 0);
}
vec4 anchorOf(int e) {
  vec4 a = anchorTexel(e, 0);
  if (a.w < 0.5) {
    return vec4(0.0);
  }
  vec4 b = anchorTexel(e, 1);
  float h = (a.w < 1.5 ? a.z * u.place.z : a.z) + b.y;
  return vec4(anchorPlace(a.xy, h), 1.0);
}
float anchorTurn(int e) {
  return anchorTexel(e, 1).x;
}
bool anchorShaped(int e) {
  return false;
}
bool anchorBehind(vec3 p) {
  return frame.size.w < 0.5 && dot(p - vec3(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
`,ws=`
vec4 anchorOf(int e) {
  vec4 m = liveOf(e);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  int j = int(w.z);
  if ((j < 0 && s.misc.w < 0.5) || m.y < 1.0) {
    return vec4(0.0);
  }
  return vec4(placeAt(w.xy, heightOf(w.xy, altNow(e)) + u.place.w), 1.0);
}
float anchorTurn(int e) {
  vec4 m = liveOf(e);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  return headingAt(m, int(w.z), s.clock.x);
}
bool anchorShaped(int e) {
  return lookOf(e).z > 0.5;
}
bool anchorBehind(vec3 p) {
  return behindGlobe(p);
}
`,Ts=/* @__PURE__ */ new Set([`trails`,`pose`,`found`]),Es=(e,t=``)=>`${mt}
const int TIME16 = ${+(e.time===`u16`)};
const int PLACE16 = ${+(e.place===`u16`)};
const int VALUE = ${e.value===`u8`?1:e.value===`f32`?2:0};
const int STEP = ${+!!e.step};
const int COLOR = ${e.color===`ramp`?2:+(e.color===`thing`)};
const int SIZE = ${e.size===`value`?2:+(e.size===`thing`)};
const int ROW = ${e.row};
const int GRID = ${e.grid};
const int FOLD = ${+!!e.fold};
const int ALT = ${e.alt===`u8`?1:e.alt===`f32`?2:0};
const int OVER_SEA = ${e.alt&&e.over===`sea`?1:0};
layout(std140) uniform S { vec4 clock; vec4 origin; vec4 box; vec4 gridA; vec4 gridB; vec4 style; vec4 color; vec4 outline; vec4 trail; vec4 trailColor; vec4 value; vec4 near; vec4 misc; vec4 grid; vec4 sz; vec4 fx; vec4 gauge; vec4 gaugeLo; vec4 gaugeMid; vec4 gaugeHi; vec4 gaugeLine; vec4 glyph; vec4 labelText; vec4 labelHalo; vec4 shadow; vec4 shadowLook; vec4 dotSize; vec4 shapeSize; vec4 pc; vec4 parts[32]; vec4 link; vec4 linkColor; vec4 linkMore; vec4 air; vec4 att; vec4 pivot; vec4 attv; } s;
${ss.map(e=>`uniform highp sampler2D ${e};`).join(`
`)}
${e.alt&&Ts.has(t)?`uniform highp sampler2D acol;`:``}
const float PI = 3.141592653589793;
const float RE = 6378137.0;
${xt}
`,Ds=`
float s2(float a) {
  float h = sin(a * 0.5);
  return 2.0 * h * h;
}
vec3 placeAt(vec2 rel, float h) {
  float dl = s.origin.x + rel.x;
  float dm = s.origin.y + rel.y;
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
bool behindGlobe(vec3 p) {
  return frame.size.w < 0.5 && dot(p - vec3(0.0, -RE, 0.0), frame.eye.xyz - p) < 0.0;
}
vec3 towardEye(vec3 p, float px) {
  vec3 toEye = frame.eye.xyz - p;
  float d = length(toEye);
  float mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(px * frame.size.z * 0.5 * mpp, d * 0.5);
}
vec2 cornerOf(int v) {
  float x = (v == 1 || v == 3) ? 1.0 : -1.0;
  float y = v >= 2 ? 1.0 : -1.0;
  return vec2(x, y);
}
vec4 metaOf(int e) {
  int i = e * 2;
  return texelFetch(things, ivec2(i % 4096, i / 4096), 0);
}
vec4 lookOf(int e) {
  int i = e * 2 + 1;
  return texelFetch(things, ivec2(i % 4096, i / 4096), 0);
}
int slotOf(vec4 m, int j) {
  if (m.z < 0.0) {
    int v = int(-m.z);
    return int(m.x) + ((int(m.w) + j) % max(v % 8192, 1)) * (v / 8192);
  }
  int cap = max(int(m.z), 1);
  return int(m.x) + (int(m.w) + j) % cap;
}
float tAt(int sl) {
  int col = sl % ROW;
  int row = sl / ROW;
  if (TIME16 == 1) {
    vec4 b = round(texelFetch(tcol, ivec2(col / 2, row), 0) * 255.0);
    vec2 p = (col % 2 == 0) ? b.xy : b.zw;
    return (p.x + p.y * 256.0) * s.misc.x;
  }
  return texelFetch(tcol, ivec2(col, row), 0).x;
}
vec2 relAt(int sl) {
  int col = sl % ROW;
  int row = sl / ROW;
  if (PLACE16 == 1) {
    vec4 b = round(texelFetch(pcol, ivec2(col, row), 0) * 255.0);
    vec2 q = vec2(b.x + b.y * 256.0, b.z + b.w * 256.0) / 65535.0;
    return s.box.xy + q * s.box.zw;
  }
  vec4 f = texelFetch(pcol, ivec2(col / 2, row), 0);
  return (col % 2 == 0) ? f.xy : f.zw;
}
float valueAt(int sl) {
  int col = sl % ROW;
  int row = sl / ROW;
  if (VALUE == 1) {
    vec4 b = texelFetch(vcol, ivec2(col / 4, row), 0);
    int c = col % 4;
    float v = c == 0 ? b.x : (c == 1 ? b.y : (c == 2 ? b.z : b.w));
    return s.value.x + v * (s.value.y - s.value.x);
  }
  if (VALUE == 2) {
    return texelFetch(vcol, ivec2(col, row), 0).x;
  }
  return 0.0;
}
int sampleAt(vec4 m, float t) {
  int n = int(m.y);
  if (n <= 0) {
    return -1;
  }
  if (t < tAt(slotOf(m, 0))) {
    return -1;
  }
  int lo = 0;
  int hi = n - 1;
  if (t >= tAt(slotOf(m, hi))) {
    return hi;
  }
  for (int k = 0; k < 24; k++) {
    if (hi - lo <= 1) {
      break;
    }
    int mid = (lo + hi) / 2;
    if (tAt(slotOf(m, mid)) <= t) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  return lo;
}
int jOf(int e) {
  vec4 b = round(texelFetch(found, ivec2(e % 4096, e / 4096), 0) * 255.0);
  return int(b.x + b.y * 256.0 + b.z * 65536.0) - 1;
}
vec4 liveOf(int e) {
  vec4 m = metaOf(e);
  if (FOLD == 1) {
    if (texelFetch(found, ivec2(e % 4096, e / 4096), 0).a < 0.5) {
      m.y = 0.0;
    }
  }
  return m;
}
vec4 whereAt(vec4 m, int j, float t) {
  int n = int(m.y);
  int sa = slotOf(m, max(j, 0));
  if (j < 0 || j >= n - 1) {
    vec2 p = relAt(sa);
    return vec4(p, float(j), 0.0);
  }
  int sb = slotOf(m, j + 1);
  float ta = tAt(sa);
  float tb = tAt(sb);
  float f = tb > ta ? clamp((t - ta) / (tb - ta), 0.0, 1.0) : 0.0;
  return vec4(mix(relAt(sa), relAt(sb), f), float(j), f);
}
float valueBetween(vec4 m, int j, float f) {
  if (VALUE == 0) {
    return 0.0;
  }
  int n = int(m.y);
  int a = clamp(j, 0, max(n - 1, 0));
  int b = min(a + 1, max(n - 1, 0));
  // (a step: held from its sample to the next, never through the values between)
  if (STEP == 1) {
    return valueAt(slotOf(m, a));
  }
  return mix(valueAt(slotOf(m, a)), valueAt(slotOf(m, b)), f);
}
float gLoad(int x, int y) {
  return texelFetch(ground, ivec2(clamp(x, 0, GRID - 1), clamp(y, 0, 2 * GRID - 1)), 0).x;
}
float gLevel(vec2 uv, int base) {
  vec2 g = clamp(uv, vec2(0.0), vec2(1.0)) * float(GRID - 1);
  ivec2 i = ivec2(floor(g));
  vec2 f = g - vec2(i);
  int x1 = min(i.x + 1, GRID - 1);
  int y1 = min(i.y + 1, GRID - 1);
  float a = mix(gLoad(i.x, base + i.y), gLoad(x1, base + i.y), f.x);
  float b = mix(gLoad(i.x, base + y1), gLoad(x1, base + y1), f.x);
  return mix(a, b, f.y);
}
float groundAt(vec2 rel) {
  vec2 b = (rel - s.gridB.xy) * s.gridB.zw;
  if (s.grid.y > 0.5 && b.x >= 0.0 && b.y >= 0.0 && b.x <= 1.0 && b.y <= 1.0) {
    return gLevel(b, GRID);
  }
  if (s.grid.x > 0.5) {
    return gLevel((rel - s.gridA.xy) * s.gridA.zw, 0);
  }
  return 0.0;
}
float heightAt(vec2 rel) {
  return groundAt(rel) * s.origin.z + s.origin.w;
}
float altAt(int sl) {
  /*@altAt*/
  return 0.0;
}
float altBetween(vec4 m, int j, float f) {
  if (ALT == 0) {
    return 0.0;
  }
  int n = int(m.y);
  int a = clamp(j, 0, max(n - 1, 0));
  int b = min(a + 1, max(n - 1, 0));
  return mix(altAt(slotOf(m, a)), altAt(slotOf(m, b)), f);
}
float timeShare(vec4 m, int j, float t) {
  int n = int(m.y);
  if (j < 0 || j >= n - 1) {
    return 0.0;
  }
  float ta = tAt(slotOf(m, j));
  float tb = tAt(slotOf(m, j + 1));
  return tb > ta ? clamp((t - ta) / (tb - ta), 0.0, 1.0) : 0.0;
}
float altNow(int e) {
  if (ALT == 0) {
    return 0.0;
  }
  int rows = (int(s.misc.z) + 4095) / 4096;
  vec4 b = round(texelFetch(found, ivec2(e % 4096, rows + e / 4096), 0) * 255.0);
  return (b.x + b.y * 256.0 + b.z * 65536.0) * 0.01 - 500.0;
}
float clearOf(vec2 rel, float alt) {
  if (ALT == 0) {
    return 0.0;
  }
  if (OVER_SEA == 1) {
    return max(alt - groundAt(rel), 0.0);
  }
  return max(alt, 0.0);
}
float heightOf(vec2 rel, float alt) {
  return heightAt(rel) + clearOf(rel, alt);
}
vec4 rampAt(float v) {
  float t = clamp((v - s.value.z) / max(s.value.w - s.value.z, 1e-9), 0.0, 1.0);
  return texelFetch(ramp, ivec2(int(t * 255.0 + 0.5), 0), 0);
}
vec4 colourOf(vec4 look, float v) {
  if (COLOR == 2) {
    return rampAt(v);
  }
  if (COLOR == 1) {
    return vec4(unpackRGB(look.x), 1.0);
  }
  return s.color;
}
vec3 gaugeColour(float f) {
  return f < 0.5 ? mix(s.gaugeLo.rgb, s.gaugeMid.rgb, f * 2.0) : mix(s.gaugeMid.rgb, s.gaugeHi.rgb, f * 2.0 - 1.0);
}
float sizeOf(vec4 look, float v) {
  if (SIZE == 2) {
    float t = clamp((v - s.sz.x) / max(s.sz.y - s.sz.x, 1e-9), 0.0, 1.0);
    return mix(s.sz.z, s.sz.w, t);
  }
  if (SIZE == 1) {
    return look.y;
  }
  return s.style.x;
}
float angleDiff(float a, float b) {
  float d = b - a;
  return d - 2.0 * PI * floor((d + PI) / (2.0 * PI));
}
float pieceHeading(vec4 m, int k) {
  int n = int(m.y);
  for (int back = 0; back < 6; back++) {
    int i = k - back;
    if (i >= 0 && i + 1 < n) {
      vec2 d = relAt(slotOf(m, i + 1)) - relAt(slotOf(m, i));
      if (dot(d, d) > 1e-20) {
        return atan(d.x, d.y);
      }
    }
  }
  return 0.0;
}
float headingAt(vec4 m, int j, float t) {
  int n = int(m.y);
  int k = clamp(j, 0, max(n - 2, 0));
  float h = pieceHeading(m, k);
  if (s.near.w <= 0.0 || n < 3) {
    return h;
  }
  float ta = tAt(slotOf(m, k));
  float tb = tAt(slotOf(m, k + 1));
  float tau = min(s.near.w, max(tb - ta, 0.001) * 0.5);
  if (k > 0 && t - ta < tau) {
    float h0 = pieceHeading(m, k - 1);
    return h0 + angleDiff(h0, h) * (0.5 + 0.5 * clamp((t - ta) / tau, 0.0, 1.0));
  }
  if (k + 2 < n && tb - t < tau) {
    float h1 = pieceHeading(m, k + 1);
    return h + angleDiff(h, h1) * 0.5 * (1.0 - clamp((tb - t) / tau, 0.0, 1.0));
  }
  return h;
}
`,Os={vertex:`
out vec2 v_corner;
out vec4 v_col;
flat out float v_id;
out float v_size;
void main() {
  int e = gl_InstanceID;
  vec4 m = liveOf(e);
  vec4 look = lookOf(e);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  int j = int(w.z);
  float v = valueBetween(m, j, w.w);
  vec2 corner = cornerOf(gl_VertexID);
  vec3 p = placeAt(w.xy, heightOf(w.xy, altNow(e)));
  float size = sizePx(sizeOf(look, v), s.dotSize, p);
  float on = (j >= 0 || s.misc.w > 0.5) ? 1.0 : 0.0;
  float d = length(frame.eye.xyz - p);
  float give = look.z > 0.5 ? 0.0 : 1.0;
  vec4 c = colourOf(look, v);
  vec3 lifted = towardEye(p, size);
  vec4 clip = frame.viewProj * vec4(lifted, 1.0);
  vec2 off = corner * size * frame.size.z / frame.size.xy * clip.w;
  bool hide = on < 0.5 || give <= 0.0 || m.y < 1.0 || behindGlobe(p);
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(vec4(clip.xy + off, clip.z - s.style.w * clip.w, clip.w));
  v_corner = corner;
  v_col = vec4(c.rgb, c.a * give);
  v_id = float(e);
  v_size = size * frame.size.z;
}
`,fragment:`
in vec2 v_corner;
in vec4 v_col;
flat in float v_id;
in float v_size;
out vec4 outColor;
void main() {
  float r = length(v_corner);
  float px = 2.0 / max(v_size, 1.0);
  float disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
  float ring = s.style.y > 0.0 ? smoothstep(1.0 - px * (s.style.y + 1.0), 1.0 - px * s.style.y, r) : 0.0;
  vec3 rgb = mix(v_col.rgb, s.outline.rgb, ring * s.outline.a);
  float alpha = v_col.a * disc * s.style.z;
  if (alpha <= 0.002) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(s.misc.y, v_id);
    return;
  }
  outColor = vec4(themed(rgb, 0.0), alpha);
}
`},ks={vertex:`
out vec4 v_col;
out float v_side;
out float v_px;
void main() {
  int K = max(int(s.clock.z), 1);
  int e = gl_InstanceID / K;
  int seg = gl_InstanceID - e * K;
  vec2 cs = cornerOf(gl_VertexID);
  vec4 m = liveOf(e);
  vec4 look = lookOf(e);
  int n = int(m.y);
  float t = s.clock.x;
  float tt = t - s.clock.y;
  int j = jOf(e);
  v_side = cs.y;
  v_px = 1.0;
  v_col = vec4(0.0);
  gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  if (j < 0 || j - seg < 0 || m.y < 1.0) {
    return;
  }
  if (seg > 0 && tAt(slotOf(m, j - seg + 1)) <= tt) {
    return;
  }
  int ia = max(j - seg, 0);
  int sa = slotOf(m, ia);
  float ta = tAt(sa);
  vec2 pa = relAt(sa);
  float va = valueAt(sa);
  float aa = altAt(sa);
  vec2 pb = pa;
  float tb = ta;
  float vb = va;
  float ab = aa;
  if (seg == 0) {
    if (j < n - 1) {
      int sn = slotOf(m, j + 1);
      float t1 = tAt(sn);
      float f = t1 > ta ? clamp((t - ta) / (t1 - ta), 0.0, 1.0) : 0.0;
      pb = mix(pa, relAt(sn), f);
      tb = mix(ta, t1, f);
      vb = mix(va, valueAt(sn), f);
      ab = mix(aa, altAt(sn), f);
    }
  } else {
    int sb = slotOf(m, ia + 1);
    pb = relAt(sb);
    tb = tAt(sb);
    vb = valueAt(sb);
    ab = altAt(sb);
  }
  // (a step: the piece's value its first sample's, to the next)
  if (STEP == 1) {
    vb = va;
  }
  bool ok = j >= 0 && j - seg >= 0 && tb > tt && (distance(pa, pb) > 1e-12 || abs(ab - aa) > 0.01);
  if (ta < tt && tb > ta) {
    float f = (tt - ta) / (tb - ta);
    pa = mix(pa, pb, f);
    va = mix(va, vb, f);
    aa = mix(aa, ab, f);
    ta = tt;
  }
  float end = cs.x;
  float side = cs.y;
  vec3 A = towardEye(placeAt(pa, heightOf(pa, aa)), s.trail.z);
  vec3 B = towardEye(placeAt(pb, heightOf(pb, ab)), s.trail.z);
  vec4 ca = frame.viewProj * vec4(A, 1.0);
  vec4 cb = frame.viewProj * vec4(B, 1.0);
  float nearW = 0.01;
  if (ca.w < nearW && cb.w < nearW) {
    ok = false;
  }
  if (ca.w < nearW) {
    ca = mix(ca, cb, (nearW - ca.w) / (cb.w - ca.w));
  }
  if (cb.w < nearW) {
    cb = mix(cb, ca, (nearW - cb.w) / (ca.w - cb.w));
  }
  vec2 halfPx = frame.size.xy * 0.5;
  vec2 sa2 = ca.xy / ca.w * halfPx;
  vec2 sb2 = cb.xy / cb.w * halfPx;
  vec2 dir = sb2 - sa2;
  float len = length(dir);
  dir = len > 1e-6 ? dir / len : vec2(1.0, 0.0);
  vec2 nrm = vec2(-dir.y, dir.x);
  float w2 = s.trail.x * frame.size.z * 0.5 + 0.5;
  vec4 c = end > 0.0 ? cb : ca;
  vec2 offPx = nrm * side * w2 + dir * end * w2;
  vec4 clip = vec4(c.xy + offPx / halfPx * c.w, c.z - s.style.w * c.w, c.w);
  float age = clamp(((end > 0.0 ? tb : ta) - tt) / max(s.clock.y, 0.001), 0.0, 1.0);
  vec4 col = s.trail.w > 0.5 ? s.trailColor : colourOf(look, end > 0.0 ? vb : va);
  float glow = s.clock.w * smoothstep(0.85, 1.0, age);
  bool hide = !ok || m.y < 1.0 || behindGlobe(end > 0.0 ? B : A);
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(clip);
  v_col = vec4(mix(col.rgb, vec3(1.0), glow * 0.5), col.a * s.trail.y * pow(age, s.fx.x));
  v_side = side;
  v_px = w2;
}
`,fragment:`
in vec4 v_col;
in float v_side;
in float v_px;
out vec4 outColor;
void main() {
  float edge = 1.0 - smoothstep(1.0 - 1.0 / max(v_px, 1.0), 1.0, abs(v_side));
  float alpha = v_col.a * edge;
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(v_col.rgb, 0.0), alpha);
}
`},As={vertex:`
uniform highp sampler2D links;
out vec4 v_col;
out float v_side;
out float v_px;
vec3 linkAt(vec2 a, vec2 b, float ha, float hb, float rise, float u) {
  vec2 p = mix(a, b, u);
  return towardEye(placeAt(p, mix(ha, hb, u) + rise * 4.0 * u * (1.0 - u)), 1.5);
}
void main() {
  int SEG = max(int(s.linkMore.x), 1);
  int e = gl_InstanceID / SEG;
  int k = gl_InstanceID - e * SEG;
  vec2 cs = cornerOf(gl_VertexID);
  vec4 m = liveOf(e);
  vec4 look = lookOf(e);
  vec4 L = texelFetch(links, ivec2(e % 4096, e / 4096), 0);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  int j = int(w.z);
  v_side = cs.y;
  v_px = 1.0;
  v_col = vec4(0.0);
  gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  if (L.w < 0.5 || m.y < 1.0 || (j < 0 && s.misc.w < 0.5)) {
    return;
  }
  float span = length((L.xy - w.xy) * vec2(RE * frame.misc.y, RE * frame.misc.y));
  float rise = s.link.y > 0.0 ? s.link.y : min(span * 0.25, 400.0);
  float hb = L.z * s.origin.z;
  float ha = heightOf(w.xy, altNow(e));
  vec3 A = linkAt(w.xy, L.xy, ha, hb, rise, float(k) / float(SEG));
  vec3 B = linkAt(w.xy, L.xy, ha, hb, rise, float(k + 1) / float(SEG));
  vec4 ca = frame.viewProj * vec4(A, 1.0);
  vec4 cb = frame.viewProj * vec4(B, 1.0);
  if (ca.w < 0.01 || cb.w < 0.01) {
    return;
  }
  vec2 halfPx = frame.size.xy * 0.5;
  vec2 sa2 = ca.xy / ca.w * halfPx;
  vec2 sb2 = cb.xy / cb.w * halfPx;
  vec2 dir = sb2 - sa2;
  float len = length(dir);
  dir = len > 1e-6 ? dir / len : vec2(1.0, 0.0);
  vec2 nrm = vec2(-dir.y, dir.x);
  float w2 = sizePx(s.link.x, s.linkMore.yzwx * vec4(1.0, 1.0, 1.0, 0.0), A) * frame.size.z * 0.5 + 0.5;
  float end = cs.x;
  vec4 c = end > 0.0 ? cb : ca;
  vec2 offPx = nrm * cs.y * w2 + dir * end * w2;
  gl_Position = clipOut(vec4(c.xy + offPx / halfPx * c.w, c.z - s.style.w * c.w, c.w));
  bool focus = look.w > 0.5;
  v_col = vec4(s.linkColor.rgb, s.linkColor.a * (focus ? s.link.w : s.link.z) * s.style.z);
  v_px = w2;
}
`,fragment:`
in vec4 v_col;
in float v_side;
in float v_px;
out vec4 outColor;
void main() {
  float edge = 1.0 - smoothstep(1.0 - 1.0 / max(v_px, 1.0), 1.0, abs(v_side));
  float alpha = v_col.a * edge;
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(v_col.rgb, 0.0), alpha);
}
`},js={vertex:`
/*@jointColumnBinding*/
/*@poseBinding*/
vec2 poseOf(int e) {
  /*@poseOf*/
  return vec2(0.0, 0.0);
}
vec3 attitude(vec3 p, vec2 pb) {
  float cb = cos(pb.y);
  float sb = sin(pb.y);
  vec3 b = vec3(p.x * cb - p.y * sb, p.x * sb + p.y * cb, p.z);
  float cp = cos(pb.x);
  float sp = sin(pb.x);
  return vec3(b.x, b.y * cp + b.z * sp, b.z * cp - b.y * sp);
}
float jointColumnAt(int sl) {
  /*@jointColumnAt*/
  return 0.0;
}
float jointColumn(vec4 m, int j, float f) {
  int n = int(m.y);
  int a = clamp(j, 0, max(n - 1, 0));
  int b = min(a + 1, max(n - 1, 0));
  return mix(jointColumnAt(slotOf(m, a)), jointColumnAt(slotOf(m, b)), j < 0 ? 0.0 : f);
}
vec3 rotAxis(vec3 p, vec3 a, float t) {
  float c = cos(t);
  return p * c + cross(a, p) * sin(t) + a * dot(a, p) * (1.0 - c);
}
float jointFormula(int k, float time, float seed) {
  /*@jointFormula*/
  return 0.0;
}
float jointValue(int k, float seed, vec4 m, int j, float f) {
  vec4 d = s.parts[k * 4 + 2];
  float kind = s.parts[k * 4].w;
  float v = 0.0;
  if (d.x == 1.0) {
    v = seed * 6.2831853 + d.y * (frame.eye.w - s.pc.y) + s.parts[k * 4 + 3].z;
  } else if (d.x == 2.0) {
    v = jointFormula(k, frame.eye.w, seed) * (kind == 3.0 ? 1.0 : 0.017453293);
  } else if (d.x == 3.0) {
    float u = jointColumn(m, j, f) * d.y;
    v = kind == 3.0 ? u : (kind == 2.0 ? u / max(s.parts[k * 4 + 1].w, 0.001) : u * 0.017453293);
  }
  if (d.z < d.w) {
    v = clamp(v, d.z, d.w);
  }
  return v;
}
void jointMove(vec3 q, vec3 n, int part, float seed, vec4 m, int j, float f, out vec3 mq, out vec3 mn) {
  mq = q;
  mn = n;
  if (part == 0) {
    return;
  }
  for (int k = 0; k < int(s.pc.x); k++) {
    if (int(s.parts[k * 4 + 3].x + 0.5) != part) {
      continue;
    }
    vec4 pv = s.parts[k * 4];
    vec3 ax = s.parts[k * 4 + 1].xyz;
    float v = jointValue(k, seed, m, j, f);
    if (pv.w == 3.0) {
      mq = mq + ax * v;
    } else {
      mq = pv.xyz + rotAxis(mq - pv.xyz, ax, v);
      mn = rotAxis(mn, ax, v);
    }
  }
}
layout(location = 0) in float ent;
layout(location = 3) in vec4 v;
layout(location = 4) in float vc;
layout(location = 5) in float card;
out vec3 v_col;
out vec3 v_n;
flat out float v_id;
void main() {
  int e = int(ent + 0.5);
  vec4 m = liveOf(e);
  vec4 look = lookOf(e);
  float t = s.clock.x;
  vec4 w = whereAt(m, jOf(e), t);
  int j = int(w.z);
  vec3 p = placeAt(w.xy, heightOf(w.xy, altNow(e)) - s.origin.w);
  float d = length(frame.eye.xyz - p);
  float a = headingAt(m, j, t) + PI;
  float c = cos(a);
  float sn = sin(a);
  float keep = s.shapeSize.w > 0.5 ? sizeKeep(s.shapeSize.x * s.near.y, vec4(1.0, s.shapeSize.y, s.shapeSize.z, 0.0), p) : 1.0;
  int ci = card >= 65536.0 ? 0 : int(card + 0.5);
  vec3 mq; vec3 mn;
  jointMove(v.xyz, unpackNormal(v.w), (ci / 512) % 16, fract(float(e) * 0.6180339887), m, j, w.w, mq, mn);
  vec3 q = mq * s.near.y * keep;
  vec3 n = mn;
  vec2 pb = poseOf(e);
  if (pb.x != 0.0 || pb.y != 0.0) {
    vec3 pv = s.pivot.xyz * s.near.y * keep;
    q = pv + attitude(q - pv, pb);
    n = attitude(n, pb);
  }
  vec3 r = vec3(q.x * c - q.z * sn, q.y, q.x * sn + q.z * c);
  bool hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0;
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(frame.viewProj * vec4(p + r, 1.0));
  v_n = vec3(n.x * c - n.z * sn, n.y, n.x * sn + n.z * c);
  bool own = card < 65536.0 && (ci % 512) >= 256;
  vec4 col = colourOf(look, valueBetween(m, j, w.w));
  v_col = (own ? vec3(1.0) : col.rgb) * unpackRGB(vc);
  v_id = float(e);
}
`,fragment:`
in vec3 v_col;
in vec3 v_n;
flat in float v_id;
out vec4 outColor;
void main() {
  vec3 n = normalize(v_n);
  bool lit = frame.sunColor.a > 0.5;
  vec3 L = lit ? normalize(frame.sun.xyz) : normalize(vec3(0.4, 0.8, 0.45));
  float diff = max(dot(n, L), 0.0);
  vec3 rgb = lit ? v_col * (frame.ambient.rgb + frame.sunColor.rgb * diff) : v_col * (0.55 + 0.45 * diff);
  if (frame.pick.x > 0.5) {
    outColor = pickOut(s.misc.y, v_id);
    return;
  }
  outColor = vec4(themed(rgb, 0.0), 1.0);
}
`},Ms={vertex:`
layout(location = 0) in float ent;
out vec2 v_uv;
out float v_fade;
vec2 shadowAt(float up) {
  if (s.shadowLook.w > 0.0 && up > s.shadowLook.w) {
    return vec2(0.0, 0.0);
  }
  float u = s.shadowLook.z > 0.0 ? clamp(up / max(s.shadowLook.z, 0.001), 0.0, 1.0) : 0.0;
  return vec2(1.0 - 0.5 * u, (1.0 - u) * (1.0 - u));
}
void main() {
  int e = int(ent + 0.5);
  vec4 m = liveOf(e);
  float t = s.clock.x;
  vec4 w = whereAt(m, jOf(e), t);
  int j = int(w.z);
  vec3 p = placeAt(w.xy, heightAt(w.xy) - s.origin.w + 0.03);
  vec2 left = shadowAt(clearOf(w.xy, altNow(e)));
  float d = length(frame.eye.xyz - p);
  float a = headingAt(m, j, t) + PI;
  float c = cos(a);
  float sn = sin(a);
  vec2 uv = cornerOf(gl_VertexID);
  float keep = s.shapeSize.w > 0.5 ? sizeKeep(s.shapeSize.x * s.near.y, vec4(1.0, s.shapeSize.y, s.shapeSize.z, 0.0), p) : 1.0;
  vec2 q = vec2(s.shadow.z + uv.x * s.shadow.x, s.shadow.w + uv.y * s.shadow.y) * s.near.y * keep * left.x;
  vec3 r = vec3(q.x * c - q.y * sn, 0.0, q.x * sn + q.y * c);
  bool hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0 || left.y <= 0.002;
  vec4 clip = frame.viewProj * vec4(towardEye(p + r, 20.0), 1.0);
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(clip);
  v_uv = uv;
  v_fade = left.y;
}
`,fragment:`
in vec2 v_uv;
in float v_fade;
out vec4 outColor;
void main() {
  float k = s.shadowLook.y;
  vec2 q = max(abs(v_uv) - vec2(k), vec2(0.0)) / max(1.0 - k, 0.001);
  float f = 1.0 - smoothstep(0.0, 1.0, length(q));
  float alpha = s.shadowLook.x * f * f * s.style.z * v_fade;
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(0.0, 0.0, 0.0, alpha);
}
`},Ns=e=>({vertex:`
out vec2 v_uv;
out float v_fill;
out vec3 v_col;
out vec2 v_px;
flat out float v_id;
flat out float v_pct;
void main() {
  int e = gl_InstanceID;
  vec4 m = liveOf(e);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  int j = int(w.z);
  float f = clamp((valueBetween(m, j, w.w) - s.fx.z) / max(s.fx.w - s.fx.z, 1e-9), 0.0, 1.0);
  vec2 corner = cornerOf(gl_VertexID);
  vec3 p0 = placeAt(w.xy, heightOf(w.xy, altNow(e)) - s.origin.w + s.gauge.z);
  float d = length(frame.eye.xyz - p0);
  bool hide = (j < 0 && s.misc.w < 0.5) || m.y < 1.0 || d > s.gauge.w || behindGlobe(p0);
  vec3 p = p0 + (frame.eye.xyz - p0) / max(d, 0.001) * min(s.fx.y, d * 0.5);
  vec4 clip = frame.viewProj * vec4(p, 1.0);
  vec2 off = corner * s.gauge.xy * frame.size.z / frame.size.xy * clip.w;
  gl_Position = hide ? vec4(2.0, 2.0, 2.0, 1.0) : clipOut(vec4(clip.xy + off, clip.z - s.style.w * clip.w, clip.w));
  v_uv = corner * 0.5 + 0.5;
  v_fill = f;
  v_col = gaugeColour(f);
  v_px = s.gauge.xy * frame.size.z;
  v_id = float(e);
  v_pct = floor(f * 100.0 + 0.5);
}
`,fragment:`${e.label?`
uniform highp sampler2D glyphs;
int digitOf(int v, int place) {
  int p = place == 2 ? 100 : (place == 1 ? 10 : 1);
  return (v / p) % 10;
}
vec2 glyphAt(int code, float x, float y) {
  if (x < 0.0 || x > s.glyph.w || y < 0.0 || y > 1.0) {
    return vec2(0.0, 0.0);
  }
  vec2 uv = vec2((float(code) * s.glyph.w + x) / (11.0 * s.glyph.w), y);
  return textureLod(glyphs, uv, 0.0).rg;
}
vec2 labelAt(vec2 q, vec2 px, int pct) {
  float h = s.labelText.w * frame.size.z;
  int n = pct >= 100 ? 4 : (pct >= 10 ? 3 : 2);
  float w = (float(n - 1) * s.glyph.x + s.glyph.y) * h;
  float x = (q.x - (px.x - w) * 0.5) / h;
  float y = ((px.y + h) * 0.5 - q.y) / h;
  int k = clamp(int(floor(x / s.glyph.x)), 0, n - 1);
  vec2 outv = vec2(0.0, 0.0);
  for (int j = max(k - 1, 0); j <= min(k + 1, n - 1); j++) {
    int code = j == n - 1 ? 10 : digitOf(pct, n - 2 - j);
    outv = max(outv, glyphAt(code, x - float(j) * s.glyph.x + s.glyph.z, y));
  }
  return outv;
}
`:``}
in vec2 v_uv;
in float v_fill;
in vec3 v_col;
in vec2 v_px;
flat in float v_id;
flat in float v_pct;
out vec4 outColor;
void main() {
  vec2 q = v_uv * v_px;
  float edge = min(min(q.x, v_px.x - q.x), min(q.y, v_px.y - q.y));
  bool line = edge < frame.size.z;
  bool full = v_uv.x <= v_fill;
  vec3 rgb = line ? s.gaugeLine.rgb : (full ? v_col : vec3(0.08, 0.1, 0.12));
  float alpha = (line ? s.gaugeLine.a : (full ? 1.0 : 0.6)) * s.style.z;${e.label?`
  vec2 t = labelAt(q, v_px, int(v_pct));
  rgb = mix(mix(rgb, s.labelHalo.rgb, t.y * s.labelHalo.a), s.labelText.rgb, t.x * s.labelText.a);
  alpha = max(alpha, max(t.x * s.labelText.a, t.y * s.labelHalo.a) * s.style.z);`:``}
  if (frame.pick.x > 0.5) {
    outColor = pickOut(s.misc.y, v_id);
    return;
  }
  outColor = vec4(themed(rgb, 0.0), alpha);
}
`}),Ps={vertex:`
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
`,fragment:e=>`${e.fold?`
uniform highp sampler2D fold;
vec4 foldAt(int k) {
  return texelFetch(fold, ivec2(k % 4096, k / 4096), 0);
}
bool foldedAt(int e, float t) {
  vec4 ix = foldAt(e);
  int n = int(ix.y);
  int a = int(ix.x);
  if (n <= 0 || foldAt(a).x > t) {
    return false;
  }
  int lo = 0;
  int hi = n - 1;
  for (int k = 0; k < 24; k++) {
    if (hi <= lo) {
      break;
    }
    int mid = (lo + hi + 1) / 2;
    if (foldAt(a + mid).x <= t) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return t < foldAt(a + lo).y;
}
`:``}
out vec4 outColor;
void main() {
  ivec2 q = ivec2(gl_FragCoord.xy);
  int rows = (int(s.misc.z) + 4095) / 4096;
  if (ALT > 0 && q.y >= rows) {
    int ea = (q.y - rows) * 4096 + q.x;
    if (ea >= int(s.misc.z)) {
      outColor = vec4(0.0);
      return;
    }
    vec4 ma = metaOf(ea);
    int ja = sampleAt(ma, s.clock.x);
    float u = clamp(round((altBetween(ma, ja, timeShare(ma, ja, s.clock.x)) + 500.0) * 100.0), 0.0, 16777215.0);
    float hi = floor(u / 65536.0);
    float mid = floor((u - hi * 65536.0) / 256.0);
    outColor = vec4(u - hi * 65536.0 - mid * 256.0, mid, hi, 255.0) / 255.0;
    return;
  }
  int e = q.y * 4096 + q.x;
  if (e >= int(s.misc.z)) {
    outColor = vec4(0.0);
    return;
  }
  int j = sampleAt(metaOf(e), s.clock.x) + 1;
  bool folded = ${e.fold?`foldedAt(e, s.clock.x)`:`false`};
  outColor = vec4(float(j % 256), float((j / 256) % 256), float(j / 65536), folded ? 0.0 : 255.0) / 255.0;
}
`},Fs={fragment:e=>{let t=e.pose,n=e=>e===`climb`||e===`turn`?1:e===`u8`?2:e===`f32`?3:0;return`
const int PITCH = ${n(t.pitch)};
const int BANK = ${n(t.bank)};
${t.pitch===`u8`||t.pitch===`f32`?`uniform highp sampler2D pitchCol;`:``}
${t.bank===`u8`||t.bank===`f32`?`uniform highp sampler2D bankCol;`:``}
out vec4 outColor;
float columnAt(vec4 b, int col, bool isByte, float lo, float hi) {
  if (!isByte) {
    return b.x;
  }
  int c = col % 4;
  float v = c == 0 ? b.x : (c == 1 ? b.y : (c == 2 ? b.z : b.w));
  return lo + v * (hi - lo);
}
float pitchAt(int sl) {
  ${t.pitch===`u8`?`return columnAt(texelFetch(pitchCol, ivec2((sl % ROW) / 4, sl / ROW), 0), sl % ROW, true, s.attv.x, s.attv.y);`:t.pitch===`f32`?`return columnAt(texelFetch(pitchCol, ivec2(sl % ROW, sl / ROW), 0), 0, false, 0.0, 0.0);`:`return 0.0;`}
}
float bankAt(int sl) {
  ${t.bank===`u8`?`return columnAt(texelFetch(bankCol, ivec2((sl % ROW) / 4, sl / ROW), 0), sl % ROW, true, s.attv.z, s.attv.w);`:t.bank===`f32`?`return columnAt(texelFetch(bankCol, ivec2(sl % ROW, sl / ROW), 0), 0, false, 0.0, 0.0);`:`return 0.0;`}
}
float metresAt(vec2 rel) {
  float lat = 2.0 * atan(exp(frame.geo.x + s.origin.y + rel.y)) - PI * 0.5;
  return RE * cos(lat);
}
float turnShare(float x, float tk, float tl, float tr) {
  if (x < tk) {
    return 0.5 * clamp(1.0 - (tk - x) / max(tl, 0.001), 0.0, 1.0);
  }
  return 0.5 + 0.5 * clamp((x - tk) / max(tr, 0.001), 0.0, 1.0);
}
vec2 pathPose(vec4 m, int j, float t) {
  int n = int(m.y);
  if (n < 2) {
    return vec2(0.0, 0.0);
  }
  float D = max(s.att.x, 0.05);
  float tw = s.near.w <= 0.0 ? D : s.near.w;
  float t0 = max(t - D, tAt(slotOf(m, 0)));
  float t1 = min(t + D, tAt(slotOf(m, n - 1)));
  int k0 = clamp(j, 0, n - 2);
  for (int i = 0; i < 24; i++) {
    if (k0 <= 0 || tAt(slotOf(m, k0)) <= t0 - tw) {
      break;
    }
    k0 = k0 - 1;
  }
  int k1 = clamp(j, 0, n - 2);
  for (int i = 0; i < 24; i++) {
    if (k1 >= n - 2 || tAt(slotOf(m, k1 + 1)) >= t1 + tw) {
      break;
    }
    k1 = k1 + 1;
  }
  float len = 0.0;
  float turn = 0.0;
  float a0 = altAt(slotOf(m, k0));
  float a1 = a0;
  float tp = tAt(slotOf(m, max(k0 - 1, 0)));
  for (int k = k0; k <= k1; k++) {
    int sa = slotOf(m, k);
    int sb = slotOf(m, k + 1);
    float ta = tAt(sa);
    float tb = tAt(sb);
    float dt = max(tb - ta, 0.0001);
    vec2 pa = relAt(sa);
    float inside = max(min(tb, t1) - max(ta, t0), 0.0);
    len = len + length(relAt(sb) - pa) * metresAt(pa) * inside / dt;
    if (t0 >= ta && t0 <= tb) {
      a0 = mix(altAt(sa), altAt(sb), clamp((t0 - ta) / dt, 0.0, 1.0));
    }
    if (t1 >= ta && t1 <= tb) {
      a1 = mix(altAt(sa), altAt(sb), clamp((t1 - ta) / dt, 0.0, 1.0));
    }
    if (k > 0) {
      float d = angleDiff(pieceHeading(m, k - 1), pieceHeading(m, k));
      float tl = min(tw, max(ta - tp, 0.001) * 0.5);
      float tr = min(tw, dt * 0.5);
      turn = turn + d * (turnShare(t1, ta, tl, tr) - turnShare(t0, ta, tl, tr));
    }
    tp = ta;
  }
  float span = max(t1 - t0, 0.001);
  return vec2(atan(a1 - a0, max(len, 0.001)), atan(((len / span) * (turn / span)) / 9.80665));
}
vec2 poseAt(vec4 m, int j, float f, float t, float up) {
  vec2 pb = vec2(0.0, 0.0);
  if (PITCH == 1 || BANK == 1) {
    pb = pathPose(m, j, t);
  }
  float pitch = PITCH == 1 ? clamp(pb.x, -s.att.y, s.att.y) : 0.0;
  float off = ALT > 0 ? smoothstep(0.0, 1.0, up / max(s.att.w, 0.001)) : 1.0;
  float bank = BANK == 1 ? clamp(pb.y, -s.att.z, s.att.z) * off : 0.0;
  int n = int(m.y);
  int a = slotOf(m, clamp(j, 0, max(n - 1, 0)));
  int b = slotOf(m, min(clamp(j, 0, max(n - 1, 0)) + 1, max(n - 1, 0)));
  if (PITCH >= 2) {
    pitch = mix(pitchAt(a), pitchAt(b), f) * 0.017453293;
  }
  if (BANK >= 2) {
    bank = mix(bankAt(a), bankAt(b), f) * 0.017453293;
  }
  return vec2(pitch, bank);
}
void main() {
  ivec2 q = ivec2(gl_FragCoord.xy);
  int e = q.y * 4096 + q.x;
  if (e >= int(s.misc.z) || lookOf(e).z < 0.5) {
    outColor = vec4(0.0, 128.0, 0.0, 128.0) / 255.0;
    return;
  }
  vec4 m = liveOf(e);
  vec4 w = whereAt(m, jOf(e), s.clock.x);
  vec2 pb = poseAt(m, int(w.z), w.w, s.clock.x, clearOf(w.xy, altNow(e)));
  vec2 u = round(clamp(pb / PI + vec2(0.5), vec2(0.0), vec2(1.0)) * 65535.0);
  vec2 hi = floor(u / 256.0);
  outColor = vec4(u.x - hi.x * 256.0, hi.x, u.y - hi.y * 256.0, hi.y) / 255.0;
}
`}};function Is(e){return Es(e)+Ds}function Ls(e,t){let n=t.alt&&Ts.has(e)?Ds.replace(`/*@altAt*/`,Rs):Ds,r=Es(t,e);if(e===`found`)return{vertex:r+Ps.vertex,fragment:r+n+Ps.fragment(t)};if(e===`pose`)return{vertex:r+Ps.vertex,fragment:r+n+Fs.fragment(t)};let i=e===`dots`?Os:e===`trails`?ks:e===`gauge`?Ns(t):e===`shadow`?Ms:e===`links`?As:js;return{vertex:r+n+(e===`models`?i.vertex.replace(`/*@jointFormula*/`,t.formula?.glsl??``).replace(`/*@jointColumnBinding*/`,t.joint===`f32`?`uniform highp sampler2D jcol;`:``).replace(`/*@jointColumnAt*/`,t.joint===`f32`?`return texelFetch(jcol, ivec2(sl % ROW, sl / ROW), 0).x;`:``).replace(`/*@poseBinding*/`,t.pose?`uniform highp sampler2D pose;`:``).replace(`/*@poseOf*/`,t.pose?`vec4 b = round(texelFetch(pose, ivec2(e % 4096, e / 4096), 0) * 255.0);
  return (vec2(b.x + b.y * 256.0, b.z + b.w * 256.0) / 65535.0 - vec2(0.5)) * PI;`:``):i.vertex),fragment:r+i.fragment}}const Rs=`int col = sl % ROW;
  int row = sl / ROW;
  if (ALT == 1) {
    vec4 b = texelFetch(acol, ivec2(col / 4, row), 0);
    int c = col % 4;
    return s.air.x + (c == 0 ? b.x : (c == 1 ? b.y : (c == 2 ? b.z : b.w))) * (s.air.y - s.air.x);
  }
  if (ALT == 2) {
    return texelFetch(acol, ivec2(col, row), 0).x;
  }`;function zs(e){let t=`${e?Is(e):mt}
layout(std140) uniform U { vec4 place; vec4 size; vec4 look; vec4 misc; } u;
${e?``:`uniform highp sampler2D places;`}
uniform highp sampler2D items;
uniform highp sampler2D atlas;
`;return{vertex:t+(e?ws:Cs)+`
out vec2 vUv;
flat out vec4 vCell;
flat out vec4 vTint;
flat out float vId;
vec2 iconCorner(int v) {
  float x = (v == 1 || v == 3) ? 1.0 : -1.0;
  float y = v >= 2 ? 1.0 : -1.0;
  return vec2(x, y);
}
vec4 iconItem(int e, int k) {
  int i = e * 2 + k;
  return texelFetch(items, ivec2(i % 4096, i / 4096), 0);
}
float iconAngle(vec3 p, float h) {
  vec4 a = frame.viewProj * vec4(p, 1.0);
  vec4 b = frame.viewProj * vec4(p + vec3(sin(h), 0.0, -cos(h)), 1.0);
  vec2 d = (b.xy / max(b.w, 0.001) - a.xy / max(a.w, 0.001)) * frame.size.xy;
  return dot(d, d) > 1e-12 ? atan(d.x, d.y) : 0.0;
}
void main() {
  int e = gl_InstanceID;
  vec4 a = anchorOf(e);
  vec4 it = iconItem(e, 0);
  vec2 corner = iconCorner(gl_VertexID);
  gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  vUv = vec2(0.0);
  vCell = vec4(0.0);
  vTint = vec4(0.0);
  vId = float(e);
  if (a.w < 0.5 || it.x < 0.0 || anchorShaped(e) || anchorBehind(a.xyz)) {
    return;
  }
  float size = sizePx(it.y, u.size, a.xyz);
  float ang = u.look.w > 0.5 ? iconAngle(a.xyz, anchorTurn(e)) : 0.0;
  float c = cos(ang);
  float sn = sin(ang);
  vec2 r = vec2(corner.x * c + corner.y * sn, -corner.x * sn + corner.y * c);
  vec4 clip = frame.viewProj * vec4(a.xyz, 1.0);
  if (clip.w < 0.01) {
    return;
  }
  vec2 off = r * size * frame.size.z / frame.size.xy * clip.w;
  gl_Position = clipOut(vec4(clip.xy + off, clip.z, clip.w));
  vUv = vec2(corner.x * 0.5 + 0.5, 0.5 - corner.y * 0.5);
  float col = mod(it.x, u.look.y);
  float row = floor(it.x / u.look.y);
  vCell = vec4(col / u.look.y, row / u.look.z, 1.0 / u.look.y, 1.0 / u.look.z);
  vTint = iconItem(e, 1);
}
`,fragment:t+`
in vec2 vUv;
flat in vec4 vCell;
flat in vec4 vTint;
flat in float vId;
out vec4 outColor;
void main() {
  vec2 q = clamp(vUv, vec2(0.0), vec2(1.0));
  vec2 dx = dFdx(q) * 0.25;
  vec2 dy = dFdy(q) * 0.25;
  vec2 at = vCell.xy + q * vCell.zw;
  vec2 k = vCell.zw;
  vec4 c = (textureLod(atlas, at + (dx + dy) * k, 0.0) + textureLod(atlas, at + (dx - dy) * k, 0.0) + textureLod(atlas, at - (dx - dy) * k, 0.0) + textureLod(atlas, at - (dx + dy) * k, 0.0)) * 0.25;
  float alpha = c.a * vTint.a * u.look.x;
  if (alpha <= 0.004) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.misc.x, vId);
    return;
  }
  outColor = vec4(themed(c.rgb / max(c.a, 0.0001) * vTint.rgb, 0.0), alpha);
}
`}}const Bs=4096,Vs=`ASLNHST1`,Hs=65535,Us=e=>typeof e==`number`?e:typeof e==`string`?Date.parse(e):e.getTime();let Ws=0;var Gs=class e{id;attribution;epoch;timeType;timeStep;placeType;lon0;lat0;m0;box=[0,0,0,0];values;keep;travel;altitude;altMax=-1/0;ids=[];index=/* @__PURE__ */ new Map;props=[];first=/* @__PURE__ */ new Int32Array(64);count=/* @__PURE__ */ new Int32Array(64);cap=/* @__PURE__ */ new Int32Array(64);head=/* @__PURE__ */ new Int32Array(64);packed=/* @__PURE__ */ new Uint8Array(64);stride=/* @__PURE__ */ new Int32Array(64);group=null;groups=0;seen=null;slots=0;used=0;time=/* @__PURE__ */ new Float32Array;place=/* @__PURE__ */ new Float32Array;cols=[];maxSpeed=0;from=1/0;to=-1/0;version=0;shared=!1;watchers=/* @__PURE__ */ new Set;url;started=null;loaded=!1;failure=null;constructor(e={}){this.id=e.id??`log-${++Ws}`,this.attribution=e.attribution??``,this.url=e.url??null,this.timeType=e.time?.type??`f32`,this.timeStep=e.time?.step??.1,this.epoch=e.time?.epoch===void 0?NaN:Us(e.time.epoch),this.placeType=e.place?.type??`f32`,this.values=Object.entries(e.values??{}).map(([e,t])=>{let n=typeof t==`string`?{type:t}:t;return{name:e,type:n.type??`f32`,domain:n.domain??[0,1],...n.step?{step:!0}:{}}}),this.travel=!!e.travel,this.travel&&!this.values.some(e=>e.name===`travel`)&&this.values.push({name:`travel`,type:`f32`,domain:[0,1]});let t=e.altitude?typeof e.altitude==`string`?{over:e.altitude}:e.altitude:null;if(this.altitude=t?{over:t.over??`ground`,value:t.value??`alt`}:null,t&&!this.values.some(e=>e.name===this.altitude.value)&&this.values.push({name:this.altitude.value,type:t.type??`f32`,domain:t.domain??[0,1]}),t&&t.over!==void 0&&t.over!==`ground`&&t.over!==`sea`)throw new K(`E-DATA-001`,`log '${this.id}': altitude over '${String(t.over)}'`,{where:this.id,fix:`altitude: 'ground' (metres over what it stands on) or 'sea' (metres over the sea)`});if(!(this.timeStep>0))throw new K(`E-DATA-001`,`log '${this.id}': time.step must be above 0`,{where:this.id,fix:`time: { type: 'u16', step: 0.1 }`});this.keep=e.keep===1/0?1/0:Math.max(2,Math.min(8191,Math.round(e.keep??512)));let n=e.place?.box;if(this.lon0=n?(n[0]+n[2])/2:NaN,this.lat0=n?(n[1]+n[3])/2:NaN,this.m0=Number.isNaN(this.lat0)?NaN:q(this.lat0),this.placeType===`u16`){if(!n)throw new K(`E-DATA-001`,`log '${this.id}': a 16-bit place needs its box`,{where:this.id,fix:`place: { type: 'u16', box: [w, s, e, n] }`});let[e,t]=this.rel(n[0],n[1]),[r,i]=this.rel(n[2],n[3]);this.box=[e,Math.min(t,i),r,Math.max(t,i)]}}rel(e,t){return[Y(e-this.lon0)*G,q(Math.max(-85.0511,Math.min(85.0511,t)))-this.m0]}lonLat(e,t){return[Y(this.lon0+e/G),Vt(this.m0+t)]}relAt(e,t=[0,0]){let n=this.place;if(this.placeType===`u16`){let r=this.box;t[0]=r[0]+n[2*e]/Hs*(r[2]-r[0]),t[1]=r[1]+n[2*e+1]/Hs*(r[3]-r[1])}else t[0]=n[2*e],t[1]=n[2*e+1];return t}secAt(e){return this.timeType===`u16`?this.time[e]*this.timeStep:this.time[e]}valueAt(e,t){let n=this.values[e],r=this.cols[e];return n.type===`u8`?n.domain[0]+r[t]/255*(n.domain[1]-n.domain[0]):r[t]}fillTravel(){let e=this.values.findIndex(e=>e.name===`travel`);e<0&&(this.values.push({name:`travel`,type:`f32`,domain:[0,1]}),e=this.values.length-1);let t=this.cols[e];if(!(t instanceof Float32Array)||t.length<this.slots||this.shared){let n=t;t=new Float32Array(Math.max(this.slots,n?.length??0)),this.cols[e]=t}let n=[0,0],r=[0,0];for(let e=0;e<this.size;e++){let i=0;for(let a=0;a<this.count[e];a++){let o=this.slot(e,a);this.relAt(o,r),a&&(i+=Math.hypot(r[0]-n[0],r[1]-n[1])*J*Math.cos(Vt(this.m0+r[1])*G)),t[o]=i,n[0]=r[0],n[1]=r[1]}}}slot(e,t){let n=this.cap[e];return this.first[e]+(this.head[e]+t)%n*this.stride[e]}get size(){return this.ids.length}get samples(){let e=0;for(let t=0;t<this.ids.length;t++)e+=this.count[t];return e}indexOf(e){return this.index.get(String(e))??-1}get bytes(){return this.time.byteLength+this.place.byteLength+this.cols.reduce((e,t)=>e+t.byteLength,0)+this.ids.length*16}thing(e,t){let n=this.index.get(e);if(n!==void 0)return n;if(n=this.ids.length,n>=this.first.length){let e=e=>{let t=new Int32Array(e.length*2);return t.set(e),t};this.first=e(this.first),this.count=e(this.count),this.cap=e(this.cap),this.head=e(this.head),this.stride=e(this.stride);let t=new Uint8Array(this.packed.length*2);t.set(this.packed),this.packed=t}if(this.ids.push(e),this.index.set(e,n),this.props.push(t??{}),this.first[n]=0,this.count[n]=0,this.cap[n]=0,this.head[n]=0,this.packed[n]=0,this.stride[n]=1,this.keep!==1/0){let e=this.group;if(!e||e.n>=e.size){let t=Math.min(256,16*4**this.groups++);this.room(t*this.keep),e=this.group={base:this.used,size:t,n:0},this.used+=t*this.keep}this.first[n]=e.base+e.n++,this.cap[n]=this.keep,this.stride[n]=e.size}return this.emit({kind:`thing`,index:n}),n}room(e){if(this.shared&&this.unshare(),this.used+e<=this.slots)return;let t=Math.max(Bs,this.slots);for(;t<this.used+e;)t*=2;t=Math.ceil(t/Bs)*Bs;let n=this.timeType===`u16`?new Uint16Array(t):new Float32Array(t);n.set(this.time.subarray(0,this.used)),this.time=n;let r=this.placeType===`u16`?new Uint16Array(2*t):new Float32Array(2*t);r.set(this.place.subarray(0,2*this.used)),this.place=r,this.cols=this.values.map((e,n)=>{let r=e.type===`u8`?new Uint8Array(t):new Float32Array(t);return this.cols[n]&&r.set(this.cols[n].subarray(0,this.used)),r}),this.slots=t,this.emit({kind:`all`})}unshare(){this.shared=!1,this.time=this.time.slice(),this.place=this.place.slice(),this.cols=this.cols.map(e=>e.slice())}append(e,t,n,r,i,a){let o=Us(t);if(!Number.isFinite(o)||!Number.isFinite(n)||!Number.isFinite(r))return!1;Number.isNaN(this.epoch)&&(this.epoch=o),Number.isNaN(this.lon0)&&(this.lon0=n,this.lat0=r,this.m0=q(r));let s=this.thing(String(e),a),c=this.count[s],l=(o-this.epoch)/1e3,u=this.timeType===`u16`?Math.round(l/this.timeStep)*this.timeStep:l;if(c>0&&u<this.secAt(this.slot(s,c-1))-1e-6)return!1;let d=c>0?this.slot(s,c-1):-1,f=this.values.map((e,t)=>d>=0?this.cols[t][d]:0);if(this.timeType===`u16`&&(l<0||l/this.timeStep>Hs))throw new K(`E-DATA-001`,`log '${this.id}': ${new Date(o).toISOString()} is outside its 16-bit time (${this.timeStep} s steps from ${new Date(this.epoch).toISOString()})`,{where:this.id,fix:`time: { type: 'f32' } for a log that runs longer`});let p;if(c<this.cap[s]?p=this.slot(s,c):this.stride[s]>1?(p=this.slot(s,0),this.head[s]=(this.head[s]+1)%this.cap[s]):(this.grow(s,Math.max(16,this.cap[s]*2)),p=this.slot(s,this.count[s])),this.count[s]<this.cap[s]&&this.count[s]++,this.dense.length&&(this.dense=[]),this.write(p,l,n,r,i,f),c>0){let e=this.slot(s,this.count[s]-2),t=u-this.secAt(e);if(t>.05){let n=this.relAt(e),i=this.relAt(p),a=J*Math.cos(r*G),o=Math.hypot(i[0]-n[0],i[1]-n[1])*a/t;o>this.maxSpeed&&o<1e4&&(this.maxSpeed=o)}if(this.travel){let t=this.values.findIndex(e=>e.name===`travel`),n=this.cols[t],i=this.relAt(e),a=this.relAt(p);n[p]=n[e]+Math.hypot(a[0]-i[0],a[1]-i[1])*J*Math.cos(r*G)}}return o<this.from&&(this.from=o),o>this.to&&(this.to=o),this.version++,this.emit({kind:`samples`,lo:p,hi:p+1}),this.emit({kind:`thing`,index:s}),!0}grow(e,t){let n=this.count[e];this.room(t);let r=this.used;for(let t=0;t<n;t++)this.copySlot(this.slot(e,t),r+t);this.first[e]=r,this.cap[e]=t,this.head[e]=0,this.stride[e]=1,this.used+=t,n&&this.emit({kind:`samples`,lo:r,hi:r+n})}copySlot(e,t){this.time[t]=this.time[e],this.place[2*t]=this.place[2*e],this.place[2*t+1]=this.place[2*e+1];for(let n of this.cols)n[t]=n[e]}write(e,t,n,r,i,a){this.time[e]=this.timeType===`u16`?Math.round(t/this.timeStep):t;let[o,s]=this.rel(n,r),c=this.seen;if(c?(o<c[0]&&(c[0]=o),s<c[1]&&(c[1]=s),o>c[2]&&(c[2]=o),s>c[3]&&(c[3]=s)):this.seen=[o,s,o,s],this.placeType===`u16`){let t=this.box;this.place[2*e]=Ks((o-t[0])/(t[2]-t[0])),this.place[2*e+1]=Ks((s-t[1])/(t[3]-t[1]))}else this.place[2*e]=o,this.place[2*e+1]=s;if(this.values.forEach((t,n)=>{let r=i?.[t.name],o=this.cols[n];o[e]=r===void 0||!Number.isFinite(r)?a[n]:t.type===`u8`?Math.round(Math.max(0,Math.min(1,(r-t.domain[0])/(t.domain[1]-t.domain[0]||1)))*255):r}),this.altitude){let t=this.altK();if(t>=0){let n=this.valueAt(t,e);n>this.altMax&&(this.altMax=n)}}}altK(){let e=this.altitude;return e?this.values.findIndex(t=>t.name===e.value):-1}scanMax(e){if(e<0)return-1/0;let t=-1/0;for(let n=0;n<this.size;n++)for(let r=0;r<this.count[n];r++){let i=this.valueAt(e,this.slot(n,r));i>t&&(t=i)}return t}appendMany(e){this.keep===1/0&&Array.isArray(e)&&e.length>1&&this.reserve(e);let t=this.watchers;this.watchers=/* @__PURE__ */ new Set;let n=[],r=/* @__PURE__ */ new Set,i=0,a=!1;this.watchers.add(e=>{if(e.kind===`samples`){let t=n.length;t&&n[t-1]===e.lo?n[t-1]=e.hi:n.push(e.lo,e.hi)}else e.kind===`all`?a=!0:r.add(e.index)});let o=this.altitude?.value;try{for(let t of e)this.append(t.id,t.t,t.lon,t.lat,o&&t.alt!==void 0?{...t.values,[o]:t.alt}:t.values,t.props)&&i++}finally{this.watchers=t}if(a)return this.emit({kind:`all`}),this.emit({kind:`thing`,index:-1}),i;for(let e=0;e<n.length;e+=2)this.emit({kind:`samples`,lo:n[e],hi:n[e+1]});for(let e of r)this.emit({kind:`thing`,index:e});return i}reserve(e){let t=/* @__PURE__ */ new Map,n=/* @__PURE__ */ new Map;for(let r of e){let e=String(r.id),i=t.get(e);t.set(e,(i??0)+1),i===void 0&&n.set(e,r.props)}let r=0;for(let[e,n]of t)this.index.has(e)||(r+=n);r&&this.room(r);for(let[e,r]of t){let t=this.index.get(e);if(t===void 0){let t=this.thing(e,n.get(e));this.stride[t]===1&&(this.first[t]=this.used,this.cap[t]=r,this.used+=r)}else this.stride[t]===1&&this.count[t]+r>this.cap[t]&&this.grow(t,this.count[t]+r)}}watch(e){return this.watchers.add(e),()=>this.watchers.delete(e)}emit(e){for(let t of this.watchers)try{t(e)}catch(e){console.error(e)}}sampleAt(e,t){let n=this.count[e];if(!n||t<this.secAt(this.slot(e,0)))return-1;let r=0,i=n-1;if(t>=this.secAt(this.slot(e,i)))return i;for(;i-r>1;){let n=r+i>>1;this.secAt(this.slot(e,n))<=t?r=n:i=n}return r}at(e,t,n=!1){let r=n?Number(e):this.indexOf(e);if(r<0||r>=this.ids.length||!this.count[r])return null;let i=(Us(t)-this.epoch)/1e3,a=this.count[r],o=this.sampleAt(r,i),s=this.slot(r,Math.max(0,o)),c=this.slot(r,Math.min(a-1,Math.max(0,o)+1)),l=this.secAt(s),u=this.secAt(c),d=o<0?0:u>l?Math.max(0,Math.min(1,(i-l)/(u-l))):0,f=this.relAt(s),p=this.relAt(c,[0,0]),m=f[0]+(p[0]-f[0])*d,h=f[1]+(p[1]-f[1])*d,[g,_]=this.lonLat(m,h),v={};this.values.forEach((e,t)=>{let n=this.valueAt(t,s),r=this.valueAt(t,c);v[e.name]=e.step?n:n+(r-n)*d});let y=J*Math.cos(_*G),b=o>=0&&o<a-1&&u>l,x=this.altitude?.value;return{id:this.ids[r],index:r,lon:g,lat:_,heading:this.headingAt(r,Math.max(0,Math.min(a-2,o))),speed:b?Math.hypot(p[0]-f[0],p[1]-f[1])*y/(u-l):0,values:v,props:this.props[r],outside:o<0?-1:+(o>=a-1&&i>this.secAt(this.slot(r,a-1))),...x?{alt:v[x]??0}:{}}}relOf(e,t,n,r=!0,i=-1){let a=this.count[e];if(!a)return!1;let o=this.sampleAt(e,t);if(o<0&&!r)return!1;let s=this.slot(e,Math.max(0,o));if(o<0||o>=a-1)return this.relAt(s,n),i>=0&&(n[2]=this.valueAt(i,s)),!0;let c=this.slot(e,o+1),l=this.secAt(s),u=this.secAt(c),d=u>l?Math.max(0,Math.min(1,(t-l)/(u-l))):0,f=this.relAt(s,this.tmpA),p=this.relAt(c,this.tmpB);if(n[0]=f[0]+(p[0]-f[0])*d,n[1]=f[1]+(p[1]-f[1])*d,i>=0){let e=this.valueAt(i,s);n[2]=e+(this.valueAt(i,c)-e)*d}return!0}tmpA=[0,0];tmpB=[0,0];densest(e){let t=this.dense.find(([t])=>t>=e);return t?t[1]:this.densestScan(e)}dense=[];densestScan(e,t=this.ids.length){let n=0;for(let r=0;r<t;r++){let t=this.count[r],i=0;for(let a=0;a<t;a++){let t=this.secAt(this.slot(r,a));for(;this.secAt(this.slot(r,i))<=t-e;)i++;a-i+1>n&&(n=a-i+1)}}return n}densestOf(e,t){let n=this.count[e];if(!n)return 0;let r=this.sampleAt(e,this.secAt(this.slot(e,n-1))-t);return n-1-r}bounds(){return this.ids.length?(this.seen||=this.scanBounds(),this.seen?[this.seen[0],this.seen[1],this.seen[2],this.seen[3]]:null):null}scanBounds(e=this.ids.length){if(this.placeType===`u16`){let t=65535,n=65535,r=0,i=0,a=this.place;for(let o=0;o<e;o++)for(let e=0;e<this.count[o];e++){let s=this.slot(o,e),c=a[2*s],l=a[2*s+1];c<t&&(t=c),c>r&&(r=c),l<n&&(n=l),l>i&&(i=l)}let o=this.box,s=(e,t,n)=>t+e/Hs*n;return[s(t,o[0],o[2]-o[0]),s(n,o[1],o[3]-o[1]),s(r,o[0],o[2]-o[0]),s(i,o[1],o[3]-o[1])]}let t=1/0,n=1/0,r=-1/0,i=-1/0,a=this.place;for(let o=0;o<e;o++)for(let e=0;e<this.count[o];e++){let s=this.slot(o,e),c=a[2*s],l=a[2*s+1];c<t&&(t=c),c>r&&(r=c),l<n&&(n=l),l>i&&(i=l)}return Number.isFinite(t)?[t,n,r,i]:null}headingAt(e,t){let n=this.count[e],r=[0,0],i=[0,0],a=t=>{if(t<0||t+1>=n)return null;this.relAt(this.slot(e,t),r),this.relAt(this.slot(e,t+1),i);let a=i[0]-r[0],o=i[1]-r[1];return a*a+o*o>1e-20?(Math.atan2(a,o)/G+360)%360:null};for(let e=t;e>=Math.max(0,t-8);e--){let t=a(e);if(t!==null)return t}for(let e=t+1;e<Math.min(n-1,t+9);e++){let t=a(e);if(t!==null)return t}return 0}stateAt(e,t){let n=[],r=t?t.map(e=>this.indexOf(e)).filter(e=>e>=0):this.ids.map((e,t)=>t);for(let t of r){let r=this.at(t,e,!0);r&&n.push(r)}return n}dateOf(e){return new Date(this.epoch+e*1e3)}toBytes(e={}){let t=Math.min(this.ids.length,e.things??1/0),n=0;for(let e=0;e<t;e++)n+=this.count[e];let r=Math.max(Bs,Math.ceil(n/Bs)*Bs),i=new Uint32Array(t),a=new Uint32Array(t),o=this.timeType===`u16`?new Uint16Array(r):new Float32Array(r),s=this.placeType===`u16`?new Uint16Array(2*r):new Float32Array(2*r),c=this.values.map(e=>e.type===`u8`?new Uint8Array(r):new Float32Array(r)),l=0;for(let e=0;e<t;e++){i[e]=l,a[e]=this.count[e];for(let t=0;t<this.count[e];t++,l++){let n=this.slot(e,t);o[l]=this.time[n],s[2*l]=this.place[2*n],s[2*l+1]=this.place[2*n+1],c.forEach((e,t)=>e[l]=this.cols[t][n])}}let u=[{name:`start`,data:i},{name:`count`,data:a},{name:`time`,data:o},{name:`place`,data:s},...this.values.map((e,t)=>({name:`value:${e.name}`,data:c[t]}))],d=this.props.slice(0,t),f=[...new Set(d.flatMap(e=>Object.keys(e)))],p={v:1,bounds:t===this.ids.length?this.bounds():this.scanBounds(t),dense:qs.map(e=>[e,this.densestScan(e,t)]),id:this.id,attribution:this.attribution,epoch:this.epoch,origin:[this.lon0,this.lat0],time:{type:this.timeType,step:this.timeStep},place:{type:this.placeType,box:this.box},values:this.values,...this.altitude?{altitude:this.altitude,altMax:this.altMax}:{},maxSpeed:this.maxSpeed,from:this.from,to:this.to,ids:this.ids.slice(0,t),props:Object.fromEntries(f.map(e=>[e,d.map(t=>t[e]??null)])),slots:r,sections:[]},m=JSON.stringify(p),h=0;for(let e=0;e<3;e++)h=Js(12+new TextEncoder().encode(m).length+64),p.sections=u.map(e=>{let t={name:e.name,offset:h,bytes:e.data.byteLength};return h=Js(h+e.data.byteLength),t}),m=JSON.stringify(p);let g=new TextEncoder().encode(m),_=new Uint8Array(h);_.set(new TextEncoder().encode(Vs),0),new DataView(_.buffer).setUint32(8,g.length,!0),_.set(g,12);for(let e=12+g.length;e<p.sections[0].offset;e++)_[e]=32;return p.sections.forEach((e,t)=>_.set(new Uint8Array(u[t].data.buffer,u[t].data.byteOffset,u[t].data.byteLength),e.offset)),_}static fromColumns(t,n={}){let r=t.ids.length,i=t.count,a=t.lonlat??null,o=t.lon,s=t.lat,c=t.t;if(i.length!==r||!a&&(!o||!s))throw new K(`E-DATA-001`,`history.fromColumns: ${r} ids, ${i.length} counts${a||o&&s?``:`, and no places (lon and lat, or lonlat)`}`,{fix:`history.fromColumns({ ids, count, t, lon, lat })`});let l=t.start??(()=>{let e=new Float64Array(r);for(let t=1;t<r;t++)e[t]=e[t-1]+i[t-1];return e})(),u=e=>a?a[2*e]:o[e],d=e=>a?a[2*e+1]:s[e],f=0,p=1/0,m=1/0,h=-1/0,g=-1/0,_=1/0,v=-1/0;for(let e=0;e<r;e++){let t=l[e],n=t+i[e];f+=i[e];for(let e=t;e<n;e++){let t=u(e),n=d(e),r=c[e];t<p&&(p=t),t>h&&(h=t),n<m&&(m=n),n>g&&(g=n),r<_&&(_=r),r>v&&(v=r)}}if(!f)return new e({...n,keep:1/0});let y=1e-5,b=[p-y,m-y,h+y,g+y],x=!n.place||!n.time?Ys({box:b,from:_,to:v}):null,S=n.place??x.place,C=n.time??x.time,w=new e({...n,keep:1/0,place:S,time:{...C,epoch:C.epoch??_}});if(w.timeType===`u16`&&(_-w.epoch<0||(v-w.epoch)/1e3/w.timeStep>Hs))throw new K(`E-DATA-001`,`log '${w.id}': its times are outside its 16-bit time (${w.timeStep} s steps from ${new Date(w.epoch).toISOString()})`,{where:w.id,fix:`time: { type: 'f32' } for a log that runs longer`});if(Number.isNaN(w.lon0)){let e=l[r>0?(()=>{let e=0;for(;e<r-1&&!i[e];)e++;return e})():0];w.lon0=u(e),w.lat0=d(e),w.m0=q(w.lat0)}let T=Math.max(Bs,Math.ceil(f/Bs)*Bs),E=Math.max(64,r);w.time=w.timeType===`u16`?new Uint16Array(T):new Float32Array(T),w.place=w.placeType===`u16`?new Uint16Array(2*T):new Float32Array(2*T),w.cols=w.values.map(e=>e.type===`u8`?new Uint8Array(T):new Float32Array(T)),w.first=new Int32Array(E),w.count=new Int32Array(E),w.cap=new Int32Array(E),w.head=new Int32Array(E),w.stride=new Int32Array(E).fill(1),w.packed=new Uint8Array(E).fill(1,0,r),w.ids=t.ids.map(String),w.index=new Map(w.ids.map((e,t)=>[e,t])),w.props=w.ids.map((e,n)=>t.props?.[n]??{});let D=w.values.map(e=>t.values?.[e.name]??(e.name===w.altitude?.value?t.alt??null:null)),O=w.timeType===`u16`,k=w.placeType===`u16`,A=w.timeStep,j=w.epoch,M=w.box,N=1/(M[2]-M[0]||1),P=1/(M[3]-M[1]||1),F=w.lon0,I=w.m0,L=0,R=0,z=1/0,B=1/0,V=-1/0,H=-1/0,U=[0,0];for(let e=0;e<r;e++){let t=l[e],n=i[e],r=null;for(let e=t+1;e<t+n;e++)if(c[e]<c[e-1]){r=Array.from({length:n},(e,n)=>t+n).sort((e,t)=>c[e]-c[t]);break}w.first[e]=L,w.count[e]=n,w.cap[e]=n;let a=0,o=0,s=0;for(let e=0;e<n;e++,L++){let n=r?r[e]:t+e,i=(c[n]-j)/1e3,l=Y(u(n)-F)*G,f=q(Math.max(-85.0511,Math.min(85.0511,d(n))))-I;w.time[L]=O?Math.round(i/A):i,k?(w.place[2*L]=Ks((l-M[0])*N),w.place[2*L+1]=Ks((f-M[1])*P)):(w.place[2*L]=l,w.place[2*L+1]=f);for(let t=0;t<D.length;t++){let r=D[t];if(!r)continue;let i=w.values[t],a=r[n],o=w.cols[t];o[L]=Number.isFinite(a)?i.type===`u8`?Math.round(Math.max(0,Math.min(1,(a-i.domain[0])/(i.domain[1]-i.domain[0]||1)))*255):a:e?o[L-1]:0}l<z&&(z=l),l>V&&(V=l),f<B&&(B=f),f>H&&(H=f);let p=O?Math.round(i/A)*A:i,m=w.relAt(L,U);if(e&&p-s>.05){let e=J*Math.cos(d(n)*G),t=Math.hypot(m[0]-a,m[1]-o)*e/(p-s);t>R&&t<1e4&&(R=t)}a=m[0],o=m[1],s=p}}return w.slots=T,w.used=f,w.maxSpeed=R,w.from=_,w.to=v,w.seen=[z,B,V,H],w.altitude&&(w.altMax=w.scanMax(w.altK())),w.travel&&w.fillTravel(),w.version++,w}static fromBytes(t,n={}){let r=t instanceof Uint8Array?t:new Uint8Array(t);if(new TextDecoder().decode(r.subarray(0,8))!==Vs)throw new K(`E-DATA-002`,`not a saved log (its first bytes are not ASLNHST1)`,{fix:`save it with log.toBytes()`});let i=new DataView(r.buffer,r.byteOffset,r.byteLength).getUint32(8,!0),a=JSON.parse(new TextDecoder().decode(r.subarray(12,12+i)));if(a.v!==void 0&&a.v!==1)throw new K(`E-DATA-004`,`unsupported history log version '${a.v}' (expected 1)`,{fix:`use a version 1 log saved with log.toBytes()`});let o=new e({id:a.id,attribution:a.attribution,time:{type:a.time.type,step:a.time.step,epoch:a.epoch},place:{type:`f32`},values:Object.fromEntries(a.values.map(e=>[e.name,{type:e.type,domain:e.domain,...e.step?{step:!0}:{}}])),...a.altitude?{altitude:a.altitude}:n.altitude&&a.values.some(e=>e.name===(typeof n.altitude==`string`?`alt`:n.altitude.value??`alt`))?{altitude:n.altitude}:{},keep:n.keep});o.placeType=a.place.type,o.box=a.place.box,o.lon0=a.origin[0],o.lat0=a.origin[1],o.m0=q(a.origin[1]);let s=e=>{let t=a.sections.find(t=>t.name===e);if(!t)throw new K(`E-DATA-002`,`saved log '${a.id}' has no '${e}'`,{fix:`save it again with log.toBytes()`});return t},c=(e,t)=>{let n=s(e),i=r.byteOffset+n.offset;return i%t.BYTES_PER_ELEMENT===0?new t(r.buffer,i,n.bytes/t.BYTES_PER_ELEMENT):new t(r.slice(n.offset,n.offset+n.bytes).buffer,0,n.bytes/t.BYTES_PER_ELEMENT)},l=c(`start`,Uint32Array),u=c(`count`,Uint32Array),d=a.ids.length;o.ids=a.ids,o.index=new Map(a.ids.map((e,t)=>[e,t]));let f=Object.keys(a.props??{});o.props=a.ids.map((e,t)=>Object.fromEntries(f.map(e=>[e,a.props[e][t]]).filter(([,e])=>e!==null)));let p=Math.max(64,d);o.first=new Int32Array(p),o.count=new Int32Array(p),o.cap=new Int32Array(p),o.head=new Int32Array(p),o.stride=new Int32Array(p).fill(1),o.packed=new Uint8Array(p).fill(1,0,d);let m=0;for(let e=0;e<d;e++)o.first[e]=l[e],o.count[e]=u[e],o.cap[e]=u[e],m=Math.max(m,l[e]+u[e]);return o.time=a.time.type===`u16`?c(`time`,Uint16Array):c(`time`,Float32Array),o.place=a.place.type===`u16`?c(`place`,Uint16Array):c(`place`,Float32Array),o.cols=a.values.map(e=>e.type===`u8`?c(`value:${e.name}`,Uint8Array):c(`value:${e.name}`,Float32Array)),o.slots=a.slots,o.used=m,o.maxSpeed=a.maxSpeed,o.from=a.from,o.to=a.to,o.seen=a.bounds??null,o.dense=a.dense??[],o.shared=!0,o.altitude&&(o.altMax=a.altMax??o.scanMax(o.altK())),n.travel&&!a.values.some(e=>e.name===`travel`)&&(o.travel=!0,o.fillTravel()),o.version++,o}start(t){return this.url?(this.started??=(async()=>{let n=await t.loader.get(this.url,{kind:`bytes`});if(!n)throw new K(`E-DATA-002`,`nothing at ${this.url}`,{where:this.id,fix:`check the URL (a saved log: tools write it once)`});let r=e.fromBytes(n,{keep:this.keep,travel:this.travel,...this.altitude?{altitude:this.altitude}:{}});for(let e of/* @__PURE__ */ `epoch.lon0.lat0.m0.box.ids.index.props.first.count.cap.head.stride.packed.seen.dense.time.place.cols.slots.used.maxSpeed.from.to.shared.altitude.altMax`.split(`.`))this[e]=r[e];this.placeType=r.placeType,this.timeType=r.timeType,this.timeStep=r.timeStep,this.values=r.values,this.attribution||=r.attribution,this.loaded=!0,this.version++,this.emit({kind:`all`}),this.emit({kind:`thing`,index:-1})})().catch(e=>{this.failure=e instanceof Error?e:Error(String(e)),t.error?.(e)}),this.started):Promise.resolve()}get ready(){return this.started??Promise.resolve()}get loading(){return!!this.url&&!this.loaded&&!this.failure}};const Ks=e=>Math.round(Math.max(0,Math.min(1,e))*Hs),qs=[15,30,45,60,90,120,180,300,600],Js=e=>Math.ceil(e/8)*8;function Ys(e){let t=e.precision?.place??.25,n=e.precision?.time??.1,r=e.box,i=111320*Math.cos((r[1]+r[3])/2*G),a=Math.max((r[2]-r[0])*i,(r[3]-r[1])*110574),o=Us(e.from),s=(Us(e.to)-o)/1e3;return{place:a/Hs<=t?{type:`u16`,box:r}:{type:`f32`},time:s/n<=Hs?{type:`u16`,epoch:o,step:n}:{type:`f32`,epoch:o}}}function Xs(e,t={}){let n=Us(t.from??(t.log&&Number.isFinite(t.log.from)?t.log.from:e.time.getTime())),r=Us(t.to??(t.log&&Number.isFinite(t.log.to)?t.log.to:n+36e5));if(!(r>n))throw new K(`E-DATA-001`,"replay: `to` must come after `from`",{fix:`replay(w, { from, to })`});let i=t.speed??1,a=!1,o=/* @__PURE__ */ new Set,s=e=>{for(let t of o)try{t(l,e)}catch(e){console.error(e)}},c=()=>e.time.getTime(),l={get playing(){return a},get speed(){return i},from:new Date(n),to:new Date(r),get time(){return new Date(c())},get progress(){return Math.max(0,Math.min(1,(c()-n)/(r-n)))},play(){return c()>=r&&i>0&&e.setTime(new Date(n)),a=!0,e.setTime(new Date(c()),{rate:i}),s(`play`),l},pause(){return a=!1,e.setTime(new Date(c()),{rate:0}),s(`pause`),l},toggle(){return a?l.pause():l.play()},setSpeed(t){return i=t,a&&e.setTimeRate(t),s(`speed`),l},seek(t,o={}){let c=o.share?n+Math.max(0,Math.min(1,Number(t)))*(r-n):Us(t);return e.setTime(new Date(Math.max(n,Math.min(r,c))),{rate:a?i:0}),s(`seek`),l},on(e){return o.add(e),()=>o.delete(e)},dispose(){u(),o.clear()}},u=e.on(`frame`,()=>{if(!a)return;let o=c();i>0&&o>=r?t.loop?(e.setTime(new Date(n+(o-n)%(r-n)),{rate:i}),s(`loop`)):(a=!1,e.setTime(new Date(r),{rate:0}),s(`end`)):i<0&&o<=n&&(t.loop?(e.setTime(new Date(r),{rate:i}),s(`loop`)):(a=!1,e.setTime(new Date(n),{rate:0}),s(`end`)))}),d=c();return d>=n&&d<=r?e.setTime(new Date(d),{rate:0}):e.setTime(new Date(n),{rate:0}),t.play&&l.play(),l}var Zs=class e{rows;version=0;byOf=/* @__PURE__ */ new Map;byTo=/* @__PURE__ */ new Map;constructor(e){this.rows=[],this.set(e)}set(e){let t=e.filter(e=>Number.isFinite(e.from)&&Number.isFinite(e.until)&&e.until>e.from).sort((e,t)=>e.from-t.from);this.rows.length=0,this.rows.push(...t),this.byOf.clear(),this.byTo.clear();for(let e of t)(this.byOf.get(e.of)??this.byOf.set(e.of,[]).get(e.of)).push(e),(this.byTo.get(e.to)??this.byTo.set(e.to,[]).get(e.to)).push(e);return this.version++,this}windowsOf(e){let t=[];for(let n of this.byOf.get(String(e))??[]){let e=t[t.length-1];e&&n.from<=e[1]?e[1]=Math.max(e[1],n.until):t.push([n.from,n.until])}return t}membersOf(e){return this.byTo.get(String(e))??[]}hostOf(e,t){let n=Us(t);for(let t of this.byOf.get(String(e))??[])if(t.from<=n&&n<t.until)return t.to;return null}heldBy(e,t){let n=Us(t);return this.membersOf(e).filter(e=>e.from<=n&&n<e.until).map(e=>e.of)}get count(){return{relations:this,count:!0}}members(e,t,n){return{relations:this,members:{agg:e,value:t,...n?{log:n}:{}}}}static of(t,n={}){let r=(e,t)=>e[t];return new e(t.map(e=>({of:String(n.of?n.of(e):r(e,`of`)),to:String(n.to?n.to(e):r(e,`to`)),from:Us(n.from?n.from(e):r(e,`from`)),until:Us(n.until?n.until(e):r(e,`until`)),kind:String((n.kind?n.kind(e):r(e,`kind`))??``)})))}static fromLog(t,n){let r=t.values.findIndex(e=>e.name===n.value),i=[];if(r<0)throw new K(`E-DATA-001`,`relations: the log has no value '${n.value}'`,{fix:`its values: ${t.values.map(e=>e.name).join(`, `)||`none`}`});for(let e=0;e<t.size;e++){let a=null;for(let o=0;o<t.count[e];o++){let s=t.slot(e,o),c=t.epoch+t.secAt(s)*1e3,l=n.to(t.valueAt(r,s),t.ids[e]),u=l==null?null:String(l);a&&a.to!==u&&(a.until=c,i.push(a),a=null),u!==null&&!a&&(a={of:t.ids[e],to:u,from:c,until:c,kind:n.kind??``})}a&&(a.until=Math.max(a.until,t.to),a.until>a.from&&i.push(a))}return new e(i)}};const Qs={log:(e={})=>new Gs(e),fromBytes:(e,t)=>Gs.fromBytes(e,t),fromColumns:(e,t)=>Gs.fromColumns(e,t),smallest:Ys,relations:Object.assign((e,t)=>Zs.of(e,t),{fromLog:(e,t)=>Zs.fromLog(e,t)})},$s={rpm:2*Math.PI/60,"deg/s":Math.PI/180,"rad/s":1,"m/s":1},ec=(e,t)=>e.rate*(t?1:$s[e.unit??`deg/s`]),tc=e=>{let t=2166136261;for(let n of String(e))t=Math.imul(t^n.charCodeAt(0),16777619);return t>>>0},nc=e=>{let t=e.shape;return t&&typeof t.make!=`function`&&Array.isArray(t.levels)?{...e,shape:{id:t.kind??`model`,make:()=>t}}:e};var rc=class extends Zt{ep;src=null;seen=-1;off=null;constructor(e){let t=nc(e);if(super({hover:{color:`#ffd166`},...t,params:void 0}),this.chunksByDefault=256,!t.shape||typeof t.shape.make!=`function`)throw new K(`E-DATA-001`,`entities '${t.id}' needs a shape (shape({ params, build }), or a made one: a model from shapes.import)`,{where:t.id,fix:`entities({ shape: bin, data })`});this.ep=t}unwatch=[];async attach(e){await super.attach(e);for(let e of[this.ep.shape,...(this.ep.attach??[]).map(e=>e.shape)]){let t=e.watch;t&&this.unwatch.push(t.call(e,()=>this.refresh()))}let t=this.ep.data;t instanceof Te?(this.src=t,this.off=t.watch(()=>this.build()),t.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.build())):this.build()}detach(){this.off?.(),this.off=null;for(let e of this.unwatch)e();this.unwatch=[],super.detach()}refresh(){return this.seen=-1,this.sigs.clear(),this.origin=null,this.build(),this}fs=[];byChunk=/* @__PURE__ */ new Map;where=null;jointsOf=/* @__PURE__ */ new Map;sigs=/* @__PURE__ */ new Map;origin=null;builtOnce=!1;build(){if(this.src&&this.src.version===this.seen)return;this.seen=this.src?.version??0;let e=(this.src?this.src.features():this.ep.data).map(e=>e.props?e:{...e,props:e.properties??{}}),t=this.ep.variation??{},n=Math.max(1,t.builds??1),r=this.ep;this.fs=e,this.where=null,this.byChunk.clear(),this.jointsOf.clear();let i=new Set(this.keys()),a=this.origin??(()=>{for(let t of e){let e=r.at?.(t)??pc(t);if(e)return[e[0],e[1]]}return null})();if(!a){for(let e of i)this.drop(e);this.sigs.clear();return}this.origin=a;let o=111320*Math.cos(a[1]*G),s=/* @__PURE__ */ new Map,l=/* @__PURE__ */ new Map;e.forEach((e,i)=>{let c=r.at?.(e)??pc(e);if(!c)return;let u=r.params?.(e)??{},d=1+tc(t.seed?t.seed(e):e.id??JSON.stringify(c))%n,f=`${d}|${JSON.stringify(u)}`,p=l.get(f);p||l.set(f,p=r.shape.make(u,d));let m=(c[0]-a[0])*o,h=-(c[1]-a[1])*110574,g=`${Math.floor(m/500)}_${Math.floor(h/500)}`,_=cc(p),v=s.get(g);v||s.set(g,v=/* @__PURE__ */ new Map);let y=v.get(_);y||v.set(_,y={made:p,idx:[],at:[]}),y.idx.push(i),y.at.push(m,h,c[2]??0)});for(let[n,o]of s){let[s,l]=n.split(`_`).map(Number),u=[],d=[],f=[...o.values()].map(n=>{let i=n.made,a=(i.pieces?.groups??[]).slice(0,4),o=r.tints?a.map(e=>r.tints[e]):[],f=o.some(Boolean),p=[],m=[];(i.pieces?.parts??[]).forEach((e,t)=>{let n=e.joint?r.joints?.[e.name]??r.joints?.[e.joint.kind]:void 0;if(!e.joint||n==null)return;let i=e.joint.kind===`slider`,a=e.joint.limits?i?e.joint.limits:[e.joint.limits[0]*Math.PI/180,e.joint.limits[1]*Math.PI/180]:void 0,o={name:e.name,part:t+1,kind:e.joint.kind,pivot:e.pivot,axis:e.joint.axis,radius:e.joint.radius,limits:a};if(typeof n==`function`)m.push({k:p.length,fn:n}),o.own=0;else if(`value`in n)throw new K(`E-DATA-001`,`entities '${this.id}': joint '${e.name}' reads a log's value ('${n.value}'): still clones have no log`,{where:this.id,fix:`a rate or a formula; or a set moved by a log (drive: drive.data(log))`});else`formula`in n?o.formula=n.formula:o.rate=ec(n,i);p.push(o)});let h=3+ +!!f,g=h+Math.ceil(m.length/2),_=n.idx.length,v=new Float32Array(_*g*4);m.forEach((e,t)=>{p[e.k].own=h*4+t*2});for(let i=0;i<_;i++){let a=e[n.idx[i]],d=tc(t.seed?t.seed(a):a.id??i),h=(d>>>8)%1e3/1e3,_=(d>>>18)%1e3/1e3,y=i*g*4,b=(r.scale?.(a)??1)*(1+(_*2-1)*(t.scale??0)),x=r.stretch?.(a)??1,S=t.color?c(r.tint?.(a)??`#ffffff`,h,t.color):W(r.tint?.(a)??`#ffffff`),C=u.length;if(u.push(n.idx[i]),v.set([n.at[i*3]-s*500,n.at[i*3+1]-l*500,n.at[i*3+2],C,b,b*x,b,((r.heading?.(a)??0)+180)*Math.PI/180,S,0,(d>>>4)%10007/10007,0],y),f)for(let e=0;e<4;e++){let t=o[e]?.(a);v[y+12+e]=t===void 0?-1:W(t)}m.forEach((e,t)=>{let n=e.fn(a),r=p[e.k],i=r.kind===`slider`,o=y+r.own;n&&`rate`in n&&(v[o]=ec(n,i),v[o+1]=n.phase===void 0?v[y+10]*2*Math.PI:i?n.phase:n.phase*Math.PI/180)})}d.push(p);let y={shape:i.levels,records:v,stride:g,radius:fc(i.bounds),size:r.size??i.size,groups:f,joints:p,emitters:i.pieces?.emitters};return[y,...(r.attach??[]).flatMap(e=>i.anchors.filter(t=>t.name===e.at||t.kind===e.at).map(t=>{let n=e.shape.make(e.params??{},1),r=`${t.at.join(`,`)}|${t.heading??0}`,i=lc.get(n);i||lc.set(n,i=/* @__PURE__ */ new Map);let a=i.get(r);return a||i.set(r,a=n.levels.map(e=>e instanceof Float32Array?dc(e,t.at,t.heading??0):e)),d.push([]),{shape:a,records:v,stride:g,radius:y.radius,size:y.size,emitters:n.pieces?.emitters.map(e=>({...e,at:uc(e.at,t.at,t.heading??0)}))}}))]}).flat();this.byChunk.set(n,Int32Array.from(u)),this.jointsOf.set(n,d),i.delete(n);let p={forms:[...o.keys()].join(`|`)+JSON.stringify(d),recs:f.map(e=>e.records)},m=this.sigs.get(n);m&&m.forms===p.forms&&m.recs.length===p.recs.length&&m.recs.every((e,t)=>ic(e,p.recs[t]))||(this.sigs.set(n,p),this.put(n,{origin:a,side:500,at:[s*500,l*500],instances:f,...this.builtOnce?{fade:!1}:{},info:e=>{let t=this.byChunk.get(n)?.[e],i=t===void 0?void 0:this.fs[t];return i?{kind:r.shape.id,id:i.id??`${n}:${e}`,props:i.props}:null}}))}for(let e of i)this.drop(e),this.sigs.delete(e);this.builtOnce=!0}joint(e,t,n){let r=n!==void 0||typeof t==`string`,i=r?t:e,a=r?n:t;for(let[t,n]of this.jointsOf)n.forEach((n,o)=>{let s=this.recordsOf(t,o);s&&n.forEach((n,c)=>{if(n.name!==i&&n.kind!==i)return;let l=n.kind===`slider`,u=a&&`rate`in a?ec(a,l):0;if(!r){if(n.own===void 0)this.setJointRate(t,o,c,u);else for(let e=0;e<s.n;e++)this.clone(t,o,e,n,u);return}if(n.own===void 0)throw new K(`E-DATA-001`,`entities '${this.id}': joint '${i}' has one driver for every clone`,{where:this.id,fix:`give each clone its own: joints: { ${i}: (d) => ({ rate: … }) }; or every clone at once: joint('${i}', { rate })`});let d=this.whereOf(String(e));d&&d[0]===t&&d[1]===o&&this.clone(t,o,d[2],n,u)})});return this}clone(e,t,n,r,i){let a=this.recordsOf(e,t),o=n*a.stride*4+r.own,s=this.setClock(e,t),c=this.clockS(),l=a.rec[o],u=a.rec[o+1]+(l-i)*(c-s),d=Math.floor(r.own/4),f=a.rec.slice(n*a.stride*4+d*4,n*a.stride*4+d*4+4);f[r.own%4]=i,f[r.own%4+1]=u,this.writeRecord(e,t,n,d,Array.from(f))}whereOf(e){if(!this.where){this.where=/* @__PURE__ */ new Map;for(let[e,t]of this.byChunk)for(let n=0;;n++){let r=this.recordsOf(e,n);if(!r)break;for(let i=0;i<r.n;i++){let a=this.fs[t[r.rec[i*r.stride*4+3]]];a&&!this.where.has(String(a.id))&&this.where.set(String(a.id),[e,n,i])}}}return this.where.get(e)}};const ic=(e,t)=>{if(e.length!==t.length)return!1;for(let n=0;n<e.length;n++)if(e[n]!==t[n]&&(e[n]===e[n]||t[n]===t[n]))return!1;return!0},ac=/* @__PURE__ */ new WeakMap,oc=/* @__PURE__ */ new WeakMap;let sc=0;function cc(e){let t=ac.get(e);return t===void 0&&(t=`${e.levels.map(e=>{if(!e)return 0;let t=oc.get(e);return t===void 0&&oc.set(e,t=++sc),t}).join(`,`)}|${JSON.stringify(e.pieces?.parts??[])}|${JSON.stringify(e.anchors)}|${JSON.stringify(e.size??null)}|${e.pieces?.groups.join(`,`)??``}|${JSON.stringify(e.pieces?.emitters??[])}`,ac.set(e,t)),t}const lc=/* @__PURE__ */ new WeakMap,uc=(e,t,n)=>{let r=n*Math.PI/180,i=Math.cos(r),a=Math.sin(r);return[e[0]*i-e[2]*a+t[0],e[1]+t[1],e[0]*a+e[2]*i+t[2]]};function dc(e,t,n){let r=Float32Array.from(e),i=n*Math.PI/180,a=Math.cos(i),o=Math.sin(i);for(let e=0;e+6<=r.length;e+=6){let[i,s,c]=uc([r[e],r[e+1],r[e+2]],t,n);if(r[e]=i,r[e+1]=s,r[e+2]=c,r[e+5]<65536&&r[e+5]%256==17)continue;let l=Wn(r[e+3]);r[e+3]=F(l[0]*a-l[2]*o,l[1],l[0]*o+l[2]*a)}return r}const fc=e=>Math.max(.05,Math.hypot(e[1][0]-e[0][0],e[1][1]-e[0][1],e[1][2]-e[0][2])/2);function pc(e){let t=e.geometry;if(!t)return null;if(t.type===`Point`)return t.coordinates;let n=[],r=e=>{Array.isArray(e)&&typeof e[0]==`number`?n.push(e):Array.isArray(e)&&e.forEach(r)};if(r(t.coordinates),!n.length)return null;let i=0,a=0;for(let[e,t]of n)i+=e,a+=t;return[i/n.length,a/n.length]}var mc=class{kind=`data`;log;source;smooth;o;seen=/* @__PURE__ */ new Map;off=null;started=null;constructor(e,t={}){if(!(e instanceof Gs)&&!(e instanceof Te))throw new K(`E-DATA-001`,`drive.data takes a log (history.log) or a source (data.stream, data.rows…)`,{fix:`drive.data(history.log({ url: '/data/fleet.hist' })) · drive.data(data.stream('wss://…'))`});this.o=t,this.source=e instanceof Te?e:null,this.log=e instanceof Gs?e:new Gs({...t.log,attribution:t.log?.attribution??e.attribution}),this.smooth=t.smooth??(this.source?250:0)}start(e){return this.started??=(async()=>{await this.log.start(e);let t=this.source;t&&(this.off=t.watch(()=>this.take(t),{now:!0}),await t.start(e),this.take(t))})(),this.started}take(e){let t=this.o,n=Date.now(),r=this.log,i=[];for(let a of e.features()){if(this.seen.get(a.id)===a)continue;this.seen.set(a.id,a);let e=a.geometry;if(!e||e.type!==`Point`)continue;let[o,s,c]=e.coordinates,l=t.time?t.time(a):a.props.t??a.props.time,u={},d=r.altitude?.value;for(let e of r.values){let n=t.values?.[e.name],r=n?n(a):Number(a.props[e.name]);!Number.isFinite(r)&&!n&&e.name===d&&(r=Number(c)),Number.isFinite(r)&&(u[e.name]=r)}i.push({id:a.id,t:l??n,lon:o,lat:s,values:u,props:a.props})}i.length&&r.appendMany(i)}stop(){this.off?.(),this.off=null}};const hc=(e,t)=>new mc(e,t),gc={linear:1,ease:2},_c=e=>!!e&&typeof e==`object`&&!Array.isArray(e)&&typeof e.value==`string`,vc=e=>!!e&&typeof e==`object`&&typeof e.value==`string`;var yc=class{main;p=null;g;d;ok=!1;failed=!1;constructor(e,t,n){this.g=e,this.d=t,this.main=e.program(t),this.main.ready.then(()=>{this.ok=!0,n?.()},()=>{this.failed=!0})}pick(){return this.p??=this.g.program({...this.d,label:`${this.d.label} pick`,blend:`none`})}destroy(){this.main.destroy(),this.p?.destroy(),this.p=null}},bc=class extends D{kind=`point`;drive;g=null;progs={};variantKey=``;found=null;foundTarget=null;foundRows=0;foundKey=``;poseTex=null;poseTarget=null;poseRows=0;poseKey=``;shapedVersion=0;ubuf=null;u=/* @__PURE__ */ new Float32Array(272);quad=null;things=null;thingsData=/* @__PURE__ */ new Float32Array;thingsCap=0;cols=null;colRows=0;colKey=``;row=4096;ground=null;rampTex=null;dummy=null;glyphs=null;labelled=!1;nearN=0;offLog=null;dirtyAll=!0;ranges=[];dirtyThings=/* @__PURE__ */ new Set;allThings=!0;valueCol=-1;jointCol=-1;altCol=-1;pitchCol=-1;bankCol=-1;nowMs=0;sec=0;constructor(e){if(super(e),!(e.drive instanceof mc))throw new K(`E-DATA-001`,`entities '${this.id}': a set moved by data needs drive: drive.data(…)`,{where:this.id,fix:`entities({ drive: drive.data(history.log({ url })) })`});this.drive=e.drive}get log(){return this.drive.log}async attach(e){await super.attach(e),this.g=e.graphics,this.offLog=this.log.watch(e=>this.changed(e)),this.g&&(this.row=this.g.caps.maxTex>=4096?4096:2048,this.ubuf=this.g.buffer(`uniform`,1088,`${this.id} set`),this.quad=this.g.buffer(`index`,Uint32Array.from([0,1,2,2,1,3]),`${this.id} quad`),this.dummy=this.g.texture({label:`${this.id} none`,width:1,height:1,format:`rgba8`,filter:`nearest`}),this.dummy.write(/* @__PURE__ */ new Uint8Array(4)),this.ground=this.g.texture({label:`${this.id} ground`,width:96,height:192,format:`r32f`,filter:`nearest`})),this.drive.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>e.world.invalidate?.())}changed(e){e.kind===`all`?(this.dirtyAll=!0,this.allThings=!0,this.kDirty=!0):e.kind===`samples`?this.ranges.push(e.lo,e.hi):e.index<0?(this.allThings=!0,this.kDirty=!0):(this.dirtyThings.add(e.index),this.kSeconds>0&&!this.kDirty&&(this.K=Math.min(96,Math.max(this.K,this.log.densestOf(e.index,this.kSeconds)+1)))),this.nearLog=!0,(this.attached?.world)?.invalidate?.()}setProps(e){let t=this.props;return super.setProps(e),`trail`in e&&(e.trail?e.trail.seconds:0)!==(t.trail?t.trail.seconds:0)&&(this.kDirty=!0),(`color`in e||`size`in e)&&(this.allThings=!0,this.variantKey=``,this.rampKey=``),(`shape`in e||`near`in e)&&(this.shapeOf=null,this.nearDirty=!0,!!e.near?.shadow!=!!t.near?.shadow&&(this.variantKey=``)),(`gauge`in e||`fold`in e)&&(this.variantKey=``),`fold`in e&&(this.foldSeen=``),(`joints`in e||`shape`in e)&&(this.jointsMade=null,this.variantKey=``),(`attitude`in e||`on`in e)&&(this.variantKey=``,this.poseKey=``),this}variant(){let e=this.props.color,t=Ot(this.props.size),n=this.props.gauge||null,r=this.log,i=[_c(e)?e.value:null,vc(t)?t.value:null,n?n.value??(_c(e)?e.value:vc(t)?t.value:r.values[0]?.name??null):null].filter(e=>!!e),a=i[0]??null;if(i.some(e=>e!==a))throw new K(`E-DATA-001`,`entities '${this.id}': its colour, size and gauge read one value (here ${[...new Set(i)].map(e=>`'${e}'`).join(`, `)})`,{where:this.id,fix:`the same value for each, or a colour or size of each thing (a function)`});let o=a===null?-1:r.values.findIndex(e=>e.name===a);if(a!==null&&o<0&&!r.loading&&r.values.length+r.size>0)throw new K(`E-DATA-001`,`entities '${this.id}': the log has no value '${a}'`,{where:this.id,fix:`its values: ${r.values.map(e=>e.name).join(`, `)||`none`}`});this.valueCol=o;let s=!!n?.label&&typeof OffscreenCanvas<`u`,c=this.setJoints(),l=c.some(e=>e.formula)?pn(c,`entities '${this.id}'`):void 0,u=[...new Set(c.filter(e=>e.value!==void 0).map(e=>e.value))];if(u.length>1)throw new K(`E-DATA-001`,`entities '${this.id}': its joints read one log value (here ${u.map(e=>`'${e}'`).join(`, `)})`,{where:this.id,fix:"the same value for each, with its own `per`"});let d=u.length?r.values.findIndex(e=>e.name===u[0]):-1;if(u.length&&!r.loading&&r.values.length+r.size>0&&(d<0||r.values[d].type!==`f32`))throw new K(`E-DATA-001`,`entities '${this.id}': its joints read '${u[0]}', ${d<0?`a value its log has not`:`a byte value (a joint reads a 32-bit one)`}`,{where:this.id,fix:u[0]===`travel`?`history.log({ travel: true }): the log keeps each sample's metres along its track`:`history.log({ values: { ${u[0]}: 'f32' } })`});this.jointCol=d;let f=r.altK();this.altCol=f;let p=this.props.attitude,m=(e,t)=>{let n=r.values.findIndex(t=>t.name===e);if(n<0&&!r.loading&&r.values.length+r.size>0)throw new K(`E-DATA-001`,`entities '${this.id}': its ${t} reads '${e}', a value its log has not`,{where:this.id,fix:`its values: ${r.values.map(e=>e.name).join(`, `)||`none`} · history.log({ values: { ${e}: 'f32' } }) (degrees)`});return n},h=typeof p?.pitch==`string`?m(p.pitch,`pitch`):-1,g=typeof p?.bank==`string`?m(p.bank,`bank`):-1;this.pitchCol=h,this.bankCol=g;let _=h>=0?r.values[h].type:p?.pitch===!0&&f>=0?`climb`:`none`,v=g>=0?r.values[g].type:p?.bank===!0?`turn`:`none`,y=this.props.shape&&(_!==`none`||v!==`none`)?{pose:{pitch:_,bank:v}}:{};return{...l?{formula:l}:{},...d>=0?{joint:`f32`}:{},time:r.timeType,place:r.placeType,...this.props.fold?{fold:!0}:{},value:o<0?`none`:r.values[o].type,...o>=0&&r.values[o].step?{step:!0}:{},color:_c(e)&&o>=0?`ramp`:typeof e==`function`?`thing`:`one`,size:vc(t)&&o>=0?`value`:typeof t==`function`?`thing`:`one`,row:this.row,grid:96,...s?{label:s}:{},...f>=0?{alt:r.values[f].type,over:r.altitude.over}:{},...y}}programs(e){let t=JSON.stringify(e)+(this.props.shape?`m`:``)+(this.props.gauge?`g`:``)+(this.props.shape&&this.props.near?.shadow?`s`:``)+(this.props.links?`l`:``);if(t===this.variantKey)return;this.variantKey=t;let n=this.g;for(let e of Object.values(this.progs))e?.destroy();this.progs={};let r=e=>e.map(e=>({name:e,kind:`2d`,sample:`unfilterable`})),i=r(ss),a=()=>{this.foundKey=``,(this.attached?.world)?.invalidate?.()},o=(t,r,o=e)=>new yc(n,{label:`set ${this.id} ${t}`,wgsl:bs(t,o),glsl:Ls(t,o),blocks:[`Frame`,`S`],textures:i,...r},a);this.progs.found=o(`found`,{textures:e.fold||e.alt?[...i,...r([...e.alt?[`acol`]:[],...e.fold?[`fold`]:[]])]:r([`things`,`tcol`]),overlay:!0,blend:`none`}),this.foundKey=``,this.poseKey=``,e.pose&&(this.progs.pose=o(`pose`,{textures:[...i,...r([...e.alt?[`acol`]:[],...this.pitchCol>=0?[`pitchCol`]:[],...this.bankCol>=0?[`bankCol`]:[]])],overlay:!0,blend:`none`})),this.progs.dots=o(`dots`,{blend:`alpha`,depth:{write:!1,test:`less`}}),this.props.links&&(this.progs.links=o(`links`,{blend:`alpha`,depth:{write:!1,test:`less`},textures:[...i,{name:`links`,kind:`2d`,sample:`unfilterable`}]})),this.progs.trails=o(`trails`,{blend:`alpha`,depth:{write:!1,test:`less`},...e.alt?{textures:[...i,...r([`acol`])]}:{}}),this.props.gauge&&(e.label&&(this.glyphs??=xc(n,`${this.id} glyphs`)),this.labelled=!!(e.label&&this.glyphs),this.progs.gauge=o(`gauge`,{blend:`alpha`,depth:{write:!1,test:`less`},...this.labelled?{textures:[...i,{name:`glyphs`,kind:`2d`,sample:`float`}]}:{}},{...e,label:this.labelled})),this.props.shape&&this.props.near?.shadow&&(this.progs.shadow=o(`shadow`,{blend:`alpha`,depth:{write:!1,test:`less`},cull:`none`,instance:{stride:1,attributes:[{name:`ent`,location:0,size:1,offset:0}]}})),this.props.shape&&(this.progs.models=o(`models`,{depth:{write:!0,test:`less`},cull:`none`,...e.joint===`f32`||e.pose?{textures:[...i,...r([...e.joint===`f32`?[`jcol`]:[],...e.pose?[`pose`]:[]])]}:{},instance:{stride:1,attributes:[{name:`ent`,location:0,size:1,offset:0}]},vertex:{stride:6,attributes:[{name:`v`,location:3,size:4,offset:0},{name:`vc`,location:4,size:1,offset:4},{name:`card`,location:5,size:1,offset:5}]}}))}columns(){let e=this.g,t=this.log,n=this.row,r=Math.max(1,Math.ceil(t.slots/n)),i=[this.valueCol,this.jointCol,this.altCol,this.pitchCol,this.bankCol].join();if(!this.cols||r!==this.colRows||this.colKey!==i||this.dirtyAll){if(!this.cols||r!==this.colRows||this.colKey!==i){this.dropColumns();let a=(e,t)=>(n,r)=>new Uint8Array(e.buffer,e.byteOffset+n*t,(r-n)*t),o=(e,t)=>(n,r)=>e.subarray(n*t,r*t),s=(t,i,a)=>e.texture({label:`${this.id} ${t}`,width:n/i,height:r,format:a,filter:`nearest`}),c=t.timeType===`u16`?{tex:s(`times`,2,`rgba8`),spt:2,view:(e,n)=>a(t.time,2)(e,n)}:{tex:s(`times`,1,`r32f`),spt:1,view:(e,n)=>o(t.time,1)(e,n)},l=t.placeType===`u16`?{tex:s(`places`,1,`rgba8`),spt:1,view:(e,n)=>a(t.place,4)(e,n)}:{tex:s(`places`,2,`rgba32f`),spt:2,view:(e,n)=>o(t.place,2)(e,n)},u=(e,n)=>{let r=e>=0?t.values[e].type:null;return r===`u8`?{tex:s(n,4,`rgba8`),spt:4,view:(n,r)=>a(t.cols[e],1)(n,r)}:r===`f32`?{tex:s(n,1,`r32f`),spt:1,view:(n,r)=>o(t.cols[e],1)(n,r)}:null},d=u(this.valueCol,`values`),f=this.jointCol,p=f>=0?{tex:s(`joint values`,1,`r32f`),spt:1,view:(e,n)=>o(t.cols[f],1)(e,n)}:null;this.cols={t:c,p:l,v:d,j:p,a:u(this.altCol,`altitudes`),xp:u(this.pitchCol,`pitches`),xb:u(this.bankCol,`banks`)},this.colRows=r,this.colKey=i}this.sweep={next:0,end:Math.min(t.slots,r*n)},this.dirtyAll=!1,this.ranges.length=0}}dropColumns(){let e=this.cols;if(e){for(let t of[e.t,e.p,e.v,e.j,e.a,e.xp,e.xb])t?.tex.destroy();this.cols=null}}sweep=null;sweepStep(){let e=this.sweep;if(!e)return!0;let t=this.row,n=performance.now();for(;e.next<e.end;){let r=Math.min(e.end,e.next+32*t);if(this.upload(e.next,r),e.next=r,performance.now()-n>3)break}return e.next>=e.end&&(this.sweep=null,this.foundKey=``,!0)}upload(e,t){let n=this.cols,r=this.row;for(let i of[n.t,n.p,n.v,n.j,n.a,n.xp,n.xb]){if(!i)continue;let n=Math.floor(e/i.spt)*i.spt,a=Math.min(this.colRows*r,Math.ceil(t/i.spt)*i.spt),o=n;for(;o<a;){let e=Math.floor(o/r),t=o-e*r;if(t===0&&a-o>=r){let t=Math.floor((a-o)/r);i.tex.write(i.view(o,o+t*r),{x:0,y:e,width:r/i.spt,height:t}),o+=t*r;continue}let n=Math.min(a,(e+1)*r);i.tex.write(i.view(o,n),{x:t/i.spt,y:e,width:(n-o)/i.spt,height:1}),o=n}}}uploadRanges(){let e=this.ranges;if(!e.length||!this.cols)return;let t=[];for(let n=0;n<e.length;n+=2)t.push([e[n],e[n+1]]);t.sort((e,t)=>e[0]-t[0]);let[n,r]=t[0];for(let e=1;e<t.length;e++){let[i,a]=t[e];i<=r+16?r=Math.max(r,a):(this.upload(n,r),n=i,r=a)}this.upload(n,r),e.length=0}thingsUpload(){let e=this.g,t=this.log,n=t.size,r=Math.max(1,Math.ceil(2*n/4096));(!this.things||r>this.thingsCap)&&(this.things?.destroy(),this.thingsCap=Math.max(r,this.thingsCap*2||1),this.things=e.texture({label:`${this.id} things`,width:4096,height:this.thingsCap,format:`rgba32f`,filter:`nearest`}),this.thingsData=new Float32Array(this.thingsCap*4096*4),this.allThings=!0,this.nearDirty=!0);let i=this.thingsData,a=e=>{let n=e*8,r=t.stride[e];i[n]=t.first[e],i[n+1]=t.count[e],i[n+2]=r>1?-(t.cap[e]+r*8192):t.cap[e],i[n+3]=t.head[e];let a=this.thingOf(e),o=this.props.color,s=Ot(this.props.size);i[n+4]=typeof o==`function`?W(Sc(o(a))):0,i[n+5]=typeof s==`function`?s(a):typeof s==`number`?s:7},o=0,s=n;if(!this.allThings){o=1/0,s=-1/0;for(let e of this.dirtyThings)e<n&&(o=Math.min(o,e),s=Math.max(s,e+1))}if(s>o){if(!this.allThings&&this.dirtyThings.size<=16)for(let e of this.dirtyThings){if(e>=n)continue;a(e);let t=2*e;this.things.write(i.subarray(e*8,e*8+8),{x:t%4096,y:Math.floor(t/4096),width:2,height:1})}else{for(let e=o;e<s;e++)a(e);let e=2*o,t=2*s;for(;e<t;){let n=Math.floor(e/4096),r=e-n*4096,a=Math.min(t,(n+1)*4096);if(r===0&&t-e>=4096){let r=Math.floor((t-e)/4096);this.things.write(i.subarray(e*4,(e+r*4096)*4),{x:0,y:n,width:4096,height:r}),e+=r*4096}else this.things.write(i.subarray(e*4,a*4),{x:r,y:n,width:a-e,height:1}),e=a}}}this.allThings=!1,this.dirtyThings.clear()}thingOf(e){return{id:this.log.ids[e],props:this.log.props[e]??{},index:e}}linkTex=null;linkData=/* @__PURE__ */ new Float32Array;linkCap=0;linksUpload(e){let t=this.g,n=this.log,r=n.size,i=this.props.links.to,a=Math.max(1,Math.ceil(r/4096));(!this.linkTex||a>this.linkCap)&&(this.linkTex?.destroy(),this.linkCap=Math.max(a,this.linkCap*2||1),this.linkTex=t.texture({label:`${this.id} links`,width:4096,height:this.linkCap,format:`rgba32f`,filter:`nearest`}),this.linkData=new Float32Array(this.linkCap*4096*4),e=null);let o=this.linkData,s=e=>{let t=i(this.thingOf(e));if(!t){o.fill(0,e*4,e*4+4);return}let r=n.rel(t[0],t[1]);o[e*4]=r[0],o[e*4+1]=r[1],o[e*4+2]=t[2]??0,o[e*4+3]=1};if(e&&e.size<=16)for(let t of e)t<r&&(s(t),this.linkTex.write(o.subarray(t*4,t*4+4),{x:t%4096,y:Math.floor(t/4096),width:1,height:1}));else{for(let e=0;e<r;e++)s(e);this.linkTex.write(o.subarray(0,a*4096*4),{x:0,y:0,width:4096,height:a})}}foldTex=null;foldSeen=``;foldWin=[];foldUpload(){let e=this.props.fold,t=this.log;if(!e||!this.g)return;let n=`${e.version}|${t.size}|${t.epoch}`;if(n===this.foldSeen&&this.foldTex)return;this.foldSeen=n;let r=t.size,i=t.ids.map(n=>e.windowsOf(n).map(([e,n])=>[(e-t.epoch)/1e3,(n-t.epoch)/1e3]));this.foldWin=i.map(e=>Float64Array.from(e.flat()));let a=r+i.reduce((e,t)=>e+t.length,0),o=Math.max(1,Math.ceil(a/4096)),s=new Float32Array(o*4096*4),c=r;i.forEach((e,t)=>{s[t*4]=c,s[t*4+1]=e.length;for(let[t,n]of e)s[c*4]=t,s[c*4+1]=n,c++}),this.foldTex?.destroy(),this.foldTex=this.g.texture({label:`${this.id} fold`,width:4096,height:o,format:`rgba32f`,filter:`nearest`}),this.foldTex.write(s),this.foundKey=``}foldedNow(e,t){let n=this.foldWin[e];if(!n)return!1;for(let e=0;e<n.length;e+=2)if(t>=n[e]&&t<n[e+1])return!0;return!1}hoverIndex=-1;focusKey=``;focused=/* @__PURE__ */ new Set;hovered(e){let t=e&&e.size?[...e][0]:null;this.hoverIndex=t===null?-1:this.log.ids.indexOf(t),(this.attached?.world)?.invalidate?.()}focusSync(e){let t=e.s.user.selection,n=`${this.hoverIndex}|${t.join(``)}`;if(n===this.focusKey||!this.things)return;this.focusKey=n;let r=/* @__PURE__ */ new Set;this.hoverIndex>=0&&r.add(this.hoverIndex);for(let e of t){let t=this.log.ids.indexOf(e);t>=0&&r.add(t)}let i=this.thingsData;for(let e of/* @__PURE__ */ new Set([...this.focused,...r])){if(e*8+8>i.length)continue;i[e*8+7]=+!!r.has(e);let t=2*e+1;this.things.write(i.subarray(t*4,t*4+4),{x:t%4096,y:Math.floor(t/4096),width:1,height:1})}this.focused=r}rampKey=``;rampUpload(){let e=this.props.color;if(!_c(e))return;let t=JSON.stringify([e.ramp??`rdylgn`,e.domain]);if(t===this.rampKey&&this.rampTex)return;this.rampKey=t;let n=ve(e.ramp??`rdylgn`),r=/* @__PURE__ */ new Uint8Array(1024);for(let e=0;e<256;e++){let t=L(n(e/255));r.set([t[0]*255,t[1]*255,t[2]*255,t[3]*255].map(Math.round),e*4)}this.rampTex??=this.g.texture({label:`${this.id} ramp`,width:256,height:1,format:`rgba8`,filter:`nearest`}),this.rampTex.write(r)}gridData=/* @__PURE__ */ new Float32Array(18432);gridBox=[null,null];gridJob=null;gridSeen=[-1,-1];gridAt=[-1e9,-1e9];setBox=null;heightFn(e,t=1/0){let n=this.props.on??`stand`,r=e.world;return n===`sea`?null:typeof n==`function`?n:n===`surface`?(e,t)=>{let n=r.heightAt(e,t);return Number.isNaN(n)?n:Math.max(n,0)}:n===`stand`&&r.standAt&&t<=1500?(e,t)=>r.standAt(e,t):(e,t)=>r.heightAt(e,t)}groundStep(e){let t=this.log;if(!this.heightFn(e)||!this.ground)return;let n=e.world.groundVersion,r=n+(e.world.standVersion??0),i=t.bounds(),a=this.setBox;if(i&&(!a||i[0]<a[0]||i[1]<a[1]||i[2]>a[2]||i[3]>a[3])){let e=(i[2]-i[0])*.1+2e-5,t=(i[3]-i[1])*.1+2e-5;this.setBox=[i[0]-e,i[1]-t,i[2]+e,i[3]+t]}if(!this.gridJob){let i=e.camera,a=J*Math.cos(i.lat*G),o=Math.max(600,Math.min(4e4,i.dist*4))/a,s=t.rel(i.lon,i.lat),c=this.gridBox[1],l=!c||Math.abs(s[0]-(c[0]+c[2])/2)>(c[2]-c[0])/4||Math.abs(s[1]-(c[1]+c[3])/2)>(c[3]-c[1])/4,u=!c||(c[2]-c[0])/o>1.6||o/(c[2]-c[0])>1.6,d=this.setBox;d&&(!this.gridBox[0]||this.gridBox[0].some((e,t)=>e!==d[t])||n!==this.gridSeen[0]&&e.now-this.gridAt[0]>1e3)?this.gridJob={level:0,box:d,row:0,data:/* @__PURE__ */ new Float32Array(9216),fn:this.heightFn(e)}:(l||u||r!==this.gridSeen[1]&&e.now-this.gridAt[1]>300)&&(this.gridJob={level:1,box:[s[0]-o/2,s[1]-o/2,s[0]+o/2,s[1]+o/2],row:0,data:/* @__PURE__ */ new Float32Array(9216),fn:this.heightFn(e,o*a)}),this.gridJob&&(this.gridSeen[this.gridJob.level]=this.gridJob.level?r:n)}let o=this.gridJob;if(!o)return;let s=performance.now(),c=this.gridData.subarray(o.level*96*96,(o.level+1)*96*96);for(;o.row<96;){let e=o.box[1]+(o.box[3]-o.box[1])*o.row/95;for(let n=0;n<96;n++){let r=o.box[0]+(o.box[2]-o.box[0])*n/95,i=t.lonLat(r,e),a=o.fn(i[0],i[1]);o.data[o.row*96+n]=Number.isFinite(a)?a:c[o.row*96+n]??0}if(o.row++,performance.now()-s>2)break}o.row>=96&&(this.gridData.set(o.data,o.level*96*96),this.ground.write(o.data,{x:0,y:o.level*96,width:96,height:96}),this.gridBox[o.level]=o.box,this.gridAt[o.level]=e.now,this.gridJob=null)}K=1;kDirty=!0;kAt=-1e9;kSeconds=-1;pieces(e){let t=this.props.trail;if(t===!1)return 0;let n=t?.seconds??60;return(this.kDirty||n!==this.kSeconds)&&(this.K=Math.min(96,this.log.densest(n)+1),this.kSeconds=n,this.kAt=e,this.kDirty=!1),this.K}shapeOf=null;nearDirty=!0;nearLog=!1;nearAt=-1e9;nearKey=``;nearD=0;nearSec=NaN;nearEye=[0,0,0];made(){let e=this.props.shape;return e?(this.shapeOf||=typeof e.make==`function`?e.make():e,this.shapeOf):null}jointsMade=null;jointsNow=[];jointClock=0;jointOffsets=[];setJoints(){let e=this.made();if(!e)return[];if(e===this.jointsMade)return this.jointsNow;this.jointsMade=e,this.jointClock=this.clockS,this.jointOffsets=[];let t=[],n=this.props.joints??{};return(e.pieces?.parts??[]).forEach((e,r)=>{let i=e.joint?n[e.name]??n[e.joint.kind]:void 0;if(!e.joint||!i||t.length>=8)return;let a=e.joint.kind===`slider`,o=e.joint.limits?a?e.joint.limits:[e.joint.limits[0]*Math.PI/180,e.joint.limits[1]*Math.PI/180]:void 0;t.push({name:e.name,part:r+1,kind:e.joint.kind,pivot:e.pivot,axis:e.joint.axis,radius:e.joint.radius,limits:o,...`formula`in i?{formula:i.formula}:`value`in i?{value:i.value,per:i.per??1}:{rate:ec(i,a)}})}),this.jointsNow=t,t}clockS=0;joint(e,t){let n=this.setJoints(),r=n.flatMap((t,n)=>t.name===e||t.kind===e?[n]:[]);if(!r.length||!t)return this;let i=()=>(this.attached?.world)?.invalidate?.();if(`value`in t&&r.every(e=>n[e].value===t.value)){for(let e of r)n[e].per=t.per??1;return this.props={...this.props,joints:{...this.props.joints,...Object.fromEntries(r.map(e=>[n[e].name,t]))}},i(),this}if(`formula`in t||`value`in t||r.some(e=>n[e].formula||n[e].value!==void 0))return this.props={...this.props,joints:{...this.props.joints,...Object.fromEntries(r.map(e=>[n[e].name,t]))}},this.jointsMade=null,this.variantKey=``,i(),this;let a=this.clockS;for(let e of r){let r=n[e],i=ec(t,r.kind===`slider`);this.jointOffsets[e]=(this.jointOffsets[e]??0)+(r.rate??0)*(a-this.jointClock)-i*(a-this.jointClock),r.rate=i}return i(),this}lv=[null,null,null];nearBufs=[{buf:null,cap:0},{buf:null,cap:0},{buf:null,cap:0}];nearNs=[0,0,0];lvOf=/* @__PURE__ */ new Uint8Array;detailSeen=1;levelChanges=0;nearSet(e){let t=this.made();if(!t||!this.g){this.nearNs.fill(0),this.nearN=0;return}let n=this.props.near??{},r=n.level!==void 0;for(let e=0;e<3;e++){let i=r?t.levels[n.level]??t.levels[0]:t.levels[e];for(let n=e;!(i instanceof Float32Array)&&n>0;n--)i=t.levels[n-1];let a=i instanceof Float32Array?i:null,o=this.lv[e];if(o&&o.arr===a)continue;o&&!this.lv.some((t,n)=>n!==e&&t?.buf===o.buf)&&o.buf.destroy();let s=a?this.lv.find(e=>e?.arr===a):null;this.lv[e]=a?s??{buf:this.g.buffer(`vertex`,a,`${this.id} shape ${e}`),n:a.length/6,arr:a}:null}let i=e.camera,a=this.log,o=n.scale??1,s=t.bounds,c=Math.max(s[1][0]-s[0][0],s[1][2]-s[0][2])*o,l=i.height/(2*Math.tan(i.fov/2)),u=n.size??t.size,d=u?bt(u):null,f=n.pixels??(d?0:12),p=Math.max(.25,Math.min(1,this.detailOf(e.s))),m=n.levels??{},h=(m.full??120)/p,g=(m.simple??25)/p,_=1+(m.margin??.2),v=i.eye,y=v[1]-i.groundY,b=e=>{let t=c*l/Math.max(e,.01);return d?Math.min(d[2],Math.max(d[1],t)):t},x=a.altK(),S=a.altitude?.over===`sea`;if(b(y-(x<0||!Number.isFinite(a.altMax)?0:Math.max(0,S?a.altMax-i.groundY:a.altMax)))<f/_||i.morph<.98){this.nearN&&this.flagShaped(null),this.nearNs.fill(0),this.nearN=0,this.nearD=0,this.nearKey=``;return}let C=Math.max(3,y*.05),w=Math.max(1,a.maxSpeed),T=Math.hypot(v[0]-this.nearEye[0],v[1]-this.nearEye[1],v[2]-this.nearEye[2]);if(this.nearLog&&performance.now()-this.nearAt>250&&(this.nearLog=!1,this.nearDirty=!0),p!==this.detailSeen&&(this.detailSeen=p,this.nearDirty=!0),!this.nearDirty&&T<C&&Math.abs(this.sec-this.nearSec)*w<C)return;this.nearDirty=!1,this.nearAt=performance.now(),this.nearSec=this.sec,this.nearEye=[v[0]??0,v[1]??0,v[2]??0];let E=J*Math.cos(i.lat*G),D=a.rel(i.lon,i.lat),O=[0,0,0],k=this.props.before===`hold`,A=a.size;if(this.lvOf.length<A){let e=new Uint8Array(Math.max(64,A*2)).fill(255);e.set(this.lvOf),this.lvOf=e}let j=this.lvOf,M=[],N=new Float32Array(A),P=[[],[],[]];for(let e=0;e<A;e++){if(!a.relOf(e,this.sec,O,k,x)){j[e]=255;continue}let t=x<0?i.groundY:S?Math.max(i.groundY,O[2]):i.groundY+Math.max(0,O[2]),n=(O[0]-D[0])*E,o=-(O[1]-D[1])*E,s=Math.hypot(n-v[0],t-v[1],o-v[2]),c=b(s),l=j[e];if(N[e]=s,l===3?c<f*_:c<f/_){l!==255&&l!==3&&this.levelChanges++,j[e]=3;continue}let u=r?0:Mt(c,l>2?-1:l,h,g,_,!1);l!==255&&l!==u&&this.levelChanges++,j[e]=u,u<2||r?M.push(e):P[2].push(e)}let F=n.max??200,I=n.farMax??4e3;if(M.length>F){M.sort((e,t)=>N[e]-N[t]);for(let e of M.splice(F))r||!this.lv[2]?j[e]=3:(j[e]=2,P[2].push(e))}for(let e of M)P[r?0:j[e]].push(e);if(P[2].length>I){P[2].sort((e,t)=>N[e]-N[t]);for(let e of P[2].splice(I))j[e]=3}let L=0;for(let e=0;e<3;e++){let t=P[e],n=this.nearBufs[e],r=Float32Array.from(t);for(let e of t)N[e]>L&&(L=N[e]);(!n.buf||n.cap<t.length)&&(n.buf?.destroy(),n.cap=Math.max(64,t.length*2),n.buf=this.g.buffer(`instance`,n.cap*4,`${this.id} level ${e}`)),t.length&&n.buf.write(r),this.nearNs[e]=t.length}this.nearD=L,this.nearN=this.nearNs[0]+this.nearNs[1]+this.nearNs[2],this.flagShaped(j)}flagShaped(e){let t=this.thingsData,n=Math.min(this.log.size,t.length/8),r=[];for(let i=0;i<n;i++){let n=e&&e[i]<3?1:0;t[i*8+6]!==n&&(t[i*8+6]=n,r.push(i))}if(!r.length||!this.things)return;this.shapedVersion++;let i=[];for(let e=16;;e*=4){i=[];for(let t of r){let n=i[i.length-1];n&&t-n[1]<=e?n[1]=t:i.push([t,t])}if(i.length<=16)break}for(let[e,n]of i){let r=2*e,i=2*n+2;for(;r<i;){let e=Math.floor(r/4096),n=r-e*4096,a=Math.min(i,(e+1)*4096);this.things.write(t.subarray(r*4,a*4),{x:n,y:e,width:a-r,height:1}),r=a}}}frame(e){let t=this.log,n=this.props;if(this.clockS=e.now/1e3,!this.ubuf||t.loading||this.log.failure||!t.size)return[];let r=this.variant();if(this.programs(r),this.rampUpload(),this.foldUpload(),this.columns(),this.uploadRanges(),!this.sweepStep())return[];(this.allThings||this.dirtyThings.size)&&(n.links&&this.linksUpload(this.allThings?null:this.dirtyThings),this.thingsUpload()),n.links&&this.focusSync(e),this.groundStep(e),this.nowMs=e.s.time.now.getTime(),this.sec=(this.nowMs-t.epoch)/1e3-this.drive.smooth/1e3;let i=this.pieces(e.now);n.shape?this.nearSet(e):this.nearN=0;let a=this.u,o=e.camera,s=n.trail===!1?null:n.trail??{},c=_c(n.color)?n.color:null,l=L(c||typeof n.color==`function`?`#ffffff`:n.color??`#38bdf8`),u=n.outline===!1?null:n.outline??{},d=L(u?.color??`#0b1220cc`),f=this.valueCol,p=f>=0?t.values[f].domain:[0,1],m=c?.domain??p,h=this.gridBox[0],g=this.gridBox[1];a.set([this.sec,s?s.seconds??60:0,i,s?s.head??.5:0],0),a.set([Y(t.lon0-o.lon)*G,t.m0-q(Math.max(-85.0511,Math.min(85.0511,o.lat))),e.world.exaggeration,n.lift??.3],4),a.set([t.box[0],t.box[1],t.box[2]-t.box[0],t.box[3]-t.box[1]],8),a.set(h?[h[0],h[1],1/(h[2]-h[0]),1/(h[3]-h[1])]:[0,0,0,0],12),a.set(g?[g[0],g[1],1/(g[2]-g[0]),1/(g[3]-g[1])]:[0,0,0,0],16);let _=Ot(n.size);a.set([typeof _==`number`?_:vc(_)?Math.max(..._.range):7,u?u.width??1.5:0,(n.opacity??1)*this.shown,(n.depthBias??2)*1e-5],20),a.set(l,24),a.set(d,28),a.set([s?.width??2.5,(s?.opacity??.9)*this.shown,2,+!!s?.color],32),a.set(L(s?.color??`#ffffff`),36),a.set([p[0],p[1],m[0],m[1]],40),a.set([this.nearD,n.near?.scale??1,+(this.nearN>0),n.near?.turn??1.2],44),a.set([t.timeStep,this.pickNo,t.size,+(n.before===`hold`)],48),a.set([+!!h,+!!g,0,0],52);let v=vc(n.size)?n.size:null,y=v?.domain??p;a.set(v?[y[0],y[1],v.range[0],v.range[1]]:[0,1,7,7],56);let b=s?typeof s.fade==`number`?s.fade:gc[s.fade??`ease`]:2,x=n.gauge||null,S=x?.domain??p,C=this.made(),w=C?C.bounds[1][1]*(n.near?.scale??1):0;a.set([Math.max(.05,b),x?.over??0,S[0],S[1]],60),a.set(x?[x.size?.[0]??28,x.size?.[1]??6,x.lift??(C?w+.9:3),x.within??(C?this.nearD:1e9)]:[0,0,0,0],64);let T=x?.colors??[`#d73027`,`#f6b93b`,`#1a9850`];a.set(L(T[0]),68),a.set(L(T[1]),72),a.set(L(T[2]),76),a.set(L(x?.outline??`#0b1220e6`),80);let E=x?.label?x.label===!0?{}:x.label:null,D=L(E?.color??`#ffffff`);a.set(this.glyphs?.m??[.5,.8,.1,1],84),a.set([D[0],D[1],D[2],E?E.size??x?.size?.[1]??6:0],88),a.set(L(E?.halo??`#05070cd9`),92);let O=n.near?.shadow,k=O?O===!0?{}:O:null,A=Math.max(1,k?.spread??1.3);if(C&&k){let e=C.bounds,t=Math.max(e[1][0]-e[0][0],e[1][2]-e[0][2])*(n.near?.scale??1);a.set([(e[1][0]-e[0][0])/2*A,(e[1][2]-e[0][2])/2*A,(e[0][0]+e[1][0])/2,(e[0][2]+e[1][2])/2],96),a.set([k.opacity??.5,.75/A,k.fade===!1?0:Math.max(.01,k.fade??6*t),Math.max(0,k.off??0)],100)}let j=n.links;if(j){let e=Ot(j.width)??1.5,t=qt(j.width,`entities '${this.id}' links.width`);a.set([typeof e==`number`?e:1.5,j.height??0,j.opacity??.15,j.focus??1],244),a.set(L(j.color??`#9ad1ff`),248),a.set([Math.max(1,Math.round(j.pieces??24)),t[0],t[1],t[2]],252)}a.set(qt(n.size,`entities '${this.id}' size`),104);let M=n.near?.size??C?.size,N=bt(M,`entities '${this.id}' near.size`);a.set([C?Math.max(C.bounds[1][0]-C.bounds[0][0],C.bounds[1][2]-C.bounds[0][2]):0,N[1],N[2],+!!M],108);let P=this.setJoints(),F={hinge:1,wheel:2,slider:3};a.set([P.length,this.jointClock,0,0],112),P.forEach((e,t)=>{let n=116+t*16,r=e.limits;a.set([e.pivot[0],e.pivot[1],e.pivot[2],F[e.kind],e.axis[0],e.axis[1],e.axis[2],e.radius??0,e.formula?2:e.value===void 0?e.rate===void 0?0:1:3,e.value===void 0?e.rate??0:e.per??1,r?r[0]:0,r?r[1]:0,e.part,-1,this.jointOffsets[t]??0,0],n)});let I=e=>e>=0?t.values[e].domain:[0,1],R=n.attitude,z=R?.max??[30,45],B=R?.pivot??[0,0,0];a.set([...I(this.altCol),0,0],256),a.set([Math.max(.05,R?.damp??2),z[0]*G,z[1]*G,this.offGround()],260),a.set([B[0],B[1],B[2],0],264),a.set([...I(this.pitchCol),...I(this.bankCol)],268),this.ubuf.write(a);let V=t.size,H=Math.max(1,Math.ceil(V/4096)),U=H*(r.alt?2:1);(!this.found||U!==this.foundRows)&&(this.foundTarget?.destroy(),this.found?.destroy(),this.found=e.graphics.texture({label:`${this.id} found`,width:4096,height:U,format:`rgba8`,filter:`nearest`}),this.foundTarget=e.graphics.layerTarget(this.found,0),this.foundRows=U,this.foundKey=``);let ee=`${this.sec}|${t.version}|${V}|${this.variantKey}|${this.foldSeen}`,W=this.dummy;ee!==this.foundKey&&this.progs.found.ok&&(e.graphics.frame([{program:this.progs.found.main,blocks:[e.frame,this.ubuf],textures:r.fold&&this.foldTex||r.alt?[this.things,this.cols.t.tex,W,W,W,W,W,...r.alt?[this.cols.a?.tex??W]:[],...r.fold&&this.foldTex?[this.foldTex]:[]]:[this.things,this.cols.t.tex],count:3}],{target:this.foundTarget,clear:[0,0,0,0]}),this.foundKey=ee);let te=[this.things,this.cols.t.tex,this.cols.p.tex,this.cols.v?.tex??this.dummy,this.ground,this.rampTex??this.dummy,this.found],ne=r.alt?[this.cols.a?.tex??this.dummy]:[];if(r.pose){if(!this.poseTex||H!==this.poseRows){this.poseTarget?.destroy(),this.poseTex?.destroy(),this.poseTex=e.graphics.texture({label:`${this.id} pose`,width:4096,height:H,format:`rgba8`,filter:`nearest`,renderable:!0});let t=new Uint8Array(4096*H*4);for(let e=1;e<t.length;e+=2)t[e]=128;this.poseTex.write(t),this.poseTarget=e.graphics.layerTarget(this.poseTex,0),this.poseRows=H,this.poseKey=``}let t=`${ee}|${this.shapedVersion}|${JSON.stringify(n.attitude)}|${n.near?.turn??1.2}`,r=this.progs.pose;if(r?.ok&&this.nearN>0&&t!==this.poseKey&&this.foundKey===ee){let n=[...this.cols.xp?[this.cols.xp.tex]:[],...this.cols.xb?[this.cols.xb.tex]:[]];e.graphics.frame([{program:r.main,blocks:[e.frame,this.ubuf],textures:[...te,...ne,...n],count:3}],{target:this.poseTarget,clear:[0,128/255,0,128/255]}),this.poseKey=t}}this.trackNow={block:this.ubuf,textures:te,variant:r,at:e.now,things:V};let re=[],ie=n.pickable!==!1;for(let t=0;t<3;t++){let n=this.nearNs[t],i=this.nearBufs[t].buf,a=this.lv[t];if(n&&i&&a&&(this.progs.shadow?.ok&&t<2&&re.push({program:this.progs.shadow.main,blocks:[e.frame,this.ubuf],textures:te,instances:i,index:this.quad,count:6,instanceCount:n}),this.progs.models)){let t=this.progs.models,o=this.cols.j||r.pose?[...te,...this.cols.j?[this.cols.j.tex]:[],...r.pose?[this.poseTex]:[]]:te;re.push({program:t.main,blocks:[e.frame,this.ubuf],textures:o,instances:i,vertices:a.buf,count:a.n,instanceCount:n,pickProgram:ie?()=>t.pick():void 0})}}s&&i>0&&re.push({program:this.progs.trails.main,blocks:[e.frame,this.ubuf],textures:ne.length?[...te,...ne]:te,index:this.quad,count:6,instanceCount:V*i});let ae=this.progs.links,oe=Math.max(1,Math.round(n.links?.pieces??24));ae?.ok&&this.linkTex&&re.push({program:ae.main,blocks:[e.frame,this.ubuf],textures:[...te,this.linkTex],index:this.quad,count:6,instanceCount:V*oe});let se=this.progs.dots;n.dots!==!1&&re.push({program:se.main,blocks:[e.frame,this.ubuf],textures:te,index:this.quad,count:6,instanceCount:V,pickProgram:ie?()=>se.pick():void 0});let ce=this.progs.gauge;return ce&&re.push({program:ce.main,blocks:[e.frame,this.ubuf],textures:this.labelled&&this.glyphs?[...te,this.glyphs.tex]:te,index:this.quad,count:6,instanceCount:V,pickProgram:ie?()=>ce.pick():void 0}),re}attitude(e,t){let n=this.log,r=n.indexOf(e);if(r<0||!n.count[r])return null;if(!this.props.attitude)return{pitch:0,bank:0};(!this.attached||this.variantKey===``)&&this.variant();let i=t===void 0?this.nowMs-this.drive.smooth:typeof t==`number`?t:t.getTime(),[a,o]=this.poseAt(r,(i-n.epoch)/1e3);return{pitch:a/G,bank:o/G}}offGround(){let e=this.made();return Math.max(.5,e?e.bounds[1][1]*(this.props.near?.scale??1):.5)}groundRel(e,t){let n=(n,r)=>{let i=Math.max(0,Math.min(1,(e-n[0])/(n[2]-n[0])))*95,a=Math.max(0,Math.min(1,(t-n[1])/(n[3]-n[1])))*95,o=Math.floor(i),s=Math.floor(a),c=i-o,l=a-s,u=Math.min(o+1,95),d=Math.min(s+1,95),f=(e,t)=>this.gridData[(r+t)*96+e];return(f(o,s)*(1-c)+f(u,s)*c)*(1-l)+(f(o,d)*(1-c)+f(u,d)*c)*l},r=this.gridBox[0],i=this.gridBox[1];return i&&e>=i[0]&&t>=i[1]&&e<=i[2]&&t<=i[3]?n(i,96):r?n(r,0):0}poseAt(e,t){let n=this.log,r=this.props.attitude??{},i=n.count[e],a=n.altK(),o=t=>n.slot(e,t),s=e=>n.secAt(o(e)),c=e=>a>=0?n.valueAt(a,o(e)):0,l=[0,0],u=[0,0],d=(e,t)=>n.relAt(o(e),t),f=e=>{for(let t=0;t<6;t++){let n=e-t;if(n>=0&&n+1<i){d(n,l),d(n+1,u);let e=u[0]-l[0],t=u[1]-l[1];if(e*e+t*t>1e-20)return Math.atan2(e,t)}}return 0},p=(e,t)=>{let n=t-e;return n-2*Math.PI*Math.floor((n+Math.PI)/(2*Math.PI))},m=(e,t,n)=>Math.max(t,Math.min(n,e)),h=(e,t,n,r)=>e<t?.5*m(1-(t-e)/Math.max(n,.001),0,1):.5+.5*m((e-t)/Math.max(r,.001),0,1),g=n.sampleAt(e,t),_=0;if(g>=0&&g<i-1){let e=s(g),n=s(g+1);_=n>e?m((t-e)/(n-e),0,1):0}let v=r.pitch===!0&&a>=0||r.bank===!0,y=[0,0];if(v&&i>=2){let e=Math.max(r.damp??2,.05),a=this.props.near?.turn??1.2,o=a<=0?e:a,_=Math.max(t-e,s(0)),v=Math.min(t+e,s(i-1)),b=m(g,0,i-2);for(let e=0;e<24&&!(b<=0||s(b)<=_-o);e++)b--;let x=m(g,0,i-2);for(let e=0;e<24&&!(x>=i-2||s(x+1)>=v+o);e++)x++;let S=0,C=0,w=c(b),T=w,E=s(Math.max(b-1,0));for(let e=b;e<=x;e++){let t=s(e),r=s(e+1),i=Math.max(r-t,1e-4);d(e,l),d(e+1,u);let a=Math.max(Math.min(r,v)-Math.max(t,_),0),g=J*Math.cos(Vt(n.m0+l[1])*G);if(S+=Math.hypot(u[0]-l[0],u[1]-l[1])*g*a/i,_>=t&&_<=r&&(w=c(e)+(c(e+1)-c(e))*m((_-t)/i,0,1)),v>=t&&v<=r&&(T=c(e)+(c(e+1)-c(e))*m((v-t)/i,0,1)),e>0){let n=p(f(e-1),f(e)),r=Math.min(o,Math.max(t-E,.001)*.5),a=Math.min(o,i*.5);C+=n*(h(v,t,r,a)-h(_,t,r,a))}E=t}let D=Math.max(v-_,.001);y=[Math.atan2(T-w,Math.max(S,.001)),Math.atan(S/D*(C/D)/9.80665)]}let b=r.max??[30,45],x=1;if(a>=0){let r=[0,0,0];n.relOf(e,t,r,!0,a);let i=m((n.altitude.over===`sea`?Math.max(r[2]-this.groundRel(r[0],r[1]),0):Math.max(r[2],0))/Math.max(this.offGround(),.001),0,1);x=i*i*(3-2*i)}let S=r.pitch===!0&&a>=0?m(y[0],-b[0]*G,b[0]*G):0,C=r.bank===!0?m(y[1],-b[1]*G,b[1]*G)*x:0,w=o(m(g,0,Math.max(i-1,0))),T=o(Math.min(m(g,0,Math.max(i-1,0))+1,Math.max(i-1,0))),E=e=>n.valueAt(e,w)+(n.valueAt(e,T)-n.valueAt(e,w))*_;return typeof r.pitch==`string`&&this.pitchCol>=0&&(S=E(this.pitchCol)*G),typeof r.bank==`string`&&this.bankCol>=0&&(C=E(this.bankCol)*G),[m(S,-Math.PI/2,Math.PI/2),m(C,-Math.PI/2,Math.PI/2)]}trackNow=null;track(){return this.trackNow}wantsFrames(e){if(this.gridJob||this.sweep||this.nearNs[0]+this.nearNs[1]>0&&this.jointsNow.some(e=>e.formula||(e.rate??0)!==0))return 1/0;let t=this.log;if(!t.size)return 0;let n=this.attached?.world?.timeRunning===!0?1:e.time.rate;if(!n)return 0;let r=Math.max(.001,(e.view.pixelSize??1)*.5);return Sn(Math.max(1,t.maxSpeed)*Math.abs(n)/r)}status(){let e=super.status();this.log.loading&&e.pending.push(`${this.id}: its log loading`),this.log.failure&&e.failed.push({what:this.id,error:this.log.failure.message});let t=this.shown>0||this.target>0;t&&this.gridJob&&e.pending.push(`${this.id}: reading the ground under its things`),t&&this.sweep&&e.pending.push(`${this.id}: uploading its columns (${Math.round(100*this.sweep.next/Math.max(1,this.sweep.end))} %)`),this.log.size&&Object.values(this.progs).some(e=>e&&!e.ok&&!e.failed)&&e.pending.push(`${this.id}: its programs building`);for(let[t,n]of Object.entries(this.progs))n?.failed&&e.failed.push({what:`${this.id} ${t}`,error:`its program could not be built`});return e.ready=!e.pending.length,e}pickInfo(e){let t=this.log;if(e<0||e>=t.size)return null;let n=this.thingOf(e);return n.state=t.at(e,this.nowMs-this.drive.smooth,!0),{element:this.id,id:n.id,props:{...n.props,...n.state?.values??{}},d:n,index:e}}readouts(){return{things:()=>this.log.size,samples:()=>this.log.samples,pieces:()=>this.K,near:()=>this.nearN,bytes:()=>this.log.bytes,levels:()=>this.nearNs.join(`/`),levelChanges:()=>this.levelChanges}}counts(){let e=this.log.size;return{things:e,samples:this.log.samples,trailPieces:this.props.trail===!1?0:e*this.K,shapes:this.nearN,vertices:e*4+(this.props.trail===!1?0:e*this.K*4)+this.nearNs.reduce((e,t,n)=>e+t*(this.lv[n]?.n??0),0)}}attribution(){return this.log.attribution?[this.log.attribution]:[]}detach(){this.offLog?.(),this.offLog=null,this.drive.stop();for(let e of Object.values(this.progs))e?.destroy();this.progs={},this.variantKey=``,this.ubuf?.destroy(),this.quad?.destroy(),this.quad=null,this.things?.destroy(),this.dropColumns(),this.poseTarget?.destroy(),this.poseTex?.destroy(),this.poseTarget=null,this.poseTex=null,this.poseRows=0,this.poseKey=``,this.linkTex?.destroy(),this.linkTex=null,this.linkCap=0,this.focusKey=``,this.ground?.destroy(),this.rampTex?.destroy(),this.dummy?.destroy();for(let e of new Set(this.lv))e?.buf.destroy();this.lv=[null,null,null];for(let e of this.nearBufs)e.buf?.destroy(),e.buf=null,e.cap=0;this.nearNs.fill(0),this.glyphs?.tex.destroy(),this.glyphs=null,this.foundTarget?.destroy(),this.found?.destroy(),this.foundTarget=null,this.found=null,this.foundRows=0,this.foundKey=``,this.foldTex?.destroy(),this.foldTex=null,this.foldSeen=``,this.trackNow=null,this.ubuf=this.things=this.ground=this.rampTex=this.dummy=null,this.cols=null,this.thingsCap=0,this.colRows=0,this.dirtyAll=!0,this.sweep=null,this.allThings=!0,this.gridBox=[null,null],this.gridJob=null,this.rampKey=``,super.detach()}};function xc(e,t){if(typeof OffscreenCanvas>`u`)return null;let n=`700 30px system-ui, -apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif`,r=new OffscreenCanvas(8,8).getContext(`2d`);if(!r)return null;r.font=n;let i=Math.max(...[...`0123456789`].map(e=>r.measureText(e).width)),a=r.measureText(`%`).width,o=Math.ceil(Math.max(i,a)+6+2),s=o*11,c=r.measureText(`0`),l=new OffscreenCanvas(s,34).getContext(`2d`,{willReadFrequently:!0}),u=new OffscreenCanvas(s,34).getContext(`2d`,{willReadFrequently:!0});if(!l||!u)return null;let d=17+(c.actualBoundingBoxAscent-c.actualBoundingBoxDescent)/2;for(let e of[l,u])e.font=n,e.textBaseline=`alphabetic`,e.fillStyle=`#ffffff`;u.strokeStyle=`#ffffff`,u.lineWidth=6,u.lineJoin=`round`,[...`0123456789%`].forEach((e,t)=>{let n=t*o+3+1;l.fillText(e,n,d),u.strokeText(e,n,d),u.fillText(e,n,d)});let f=l.getImageData(0,0,s,34).data,p=u.getImageData(0,0,s,34).data,m=new Uint8Array(s*34*4);for(let e=0;e<s*34;e++)m[e*4]=f[e*4+3],m[e*4+1]=p[e*4+3],m[e*4+3]=255;let h=e.texture({label:t,width:s,height:34,format:`rgba8`,filter:`linear`});return h.write(m),{tex:h,m:[i/34,a/34,4/34,o/34]}}const Sc=e=>`#${L(e).slice(0,3).map(e=>Math.round(e*255).toString(16).padStart(2,`0`)).join(``)}`;function Cc(e){return e.drive instanceof mc?new bc(e):new rc(e)}var wc=class{of;o;host=null;set=null;source=null;items=[];ids=[];version=0;ctx=null;offs=[];onChange=()=>{};constructor(e,t={}){if(e==null)throw new K(`E-DATA-001`,"placed on nothing: give `of` (an element id, an element, a source or an array)",{fix:`icon({ of: 'stations', … }) · cards({ of: chargers, … })`});this.of=e,this.o=t}get kind(){return this.set?`set`:`still`}attach(e,t){this.ctx=e,this.onChange=t;let n=this.of;if(typeof n==`string`){let t=e.world;if(!t.element)throw new K(`E-DATA-001`,`placed on '${n}': this world cannot find elements by id`,{fix:`give the element itself`});n=t.element(n)}if(n instanceof bc){this.host=n,this.set=n,this.offs.push(n.log.watch(e=>{(e.kind===`all`||e.kind===`thing`&&(e.index<0||e.index>=this.items.length))&&this.things()})),this.things();return}if(n instanceof Te)this.source=n;else if(Array.isArray(n))this.items=n;else{let e=n,t=e.props;if(this.host=e,t.at&&!this.o.at&&(this.o={...this.o,at:t.at}),t.data instanceof Te)this.source=t.data;else if(Array.isArray(t.data))this.items=t.data;else throw new K(`E-DATA-001`,`placed on '${e.id}': it has no features (a set, a layer or an instance set)`,{where:e.id,fix:`place it on a set of entities, a layer with data, or give a source`})}let r=this.source;r?(this.offs.push(r.watch(()=>this.features())),r.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.features())):this.features()}detach(){for(let e of this.offs)e();this.offs=[],this.tex?.destroy(),this.tex=null,this.cap=0,this.ctx=null}things(){let e=this.set.log;e.size===this.items.length&&this.items.length||(this.items=e.ids.map((t,n)=>({id:t,props:e.props[n]??{},index:n})),this.ids=e.ids.slice(),this.version++,this.onChange())}features(){let e=(this.source?this.source.features():this.items).map(e=>e.props||!e.properties?e:qe([e])[0]);this.items=e,this.ids=e.map((e,t)=>String(e.id??t));let t=e.length,n=new Float64Array(t*2),r=new Float32Array(t),i=new Float32Array(t),a=new Float32Array(t),o=new Uint8Array(t),s=this.o.at,c=this.o.lift;e.forEach((e,t)=>{let l=s?s(e):Ve(e.geometry??null);l&&Number.isFinite(l[0])&&Number.isFinite(l[1])&&(n[2*t]=l[0],n[2*t+1]=l[1],r[t]=l[2]===void 0?NaN:l[2],o[t]=1,i[t]=this.o.heading?(Number(this.o.heading(e))||0)*G:0,a[t]=typeof c==`function`?Number(c(e))||0:c??0)}),this.ll=n,this.own=r,this.turn=i,this.liftOf=a,this.ok=o;let l=o.indexOf(1);l>=0&&(this.lon0=n[2*l],this.m0=q(n[2*l+1])),this.hts=/* @__PURE__ */ new Float32Array,this.version++,this.dirty=!0,this.onChange()}ll=/* @__PURE__ */ new Float64Array;own=/* @__PURE__ */ new Float32Array;turn=/* @__PURE__ */ new Float32Array;liftOf=/* @__PURE__ */ new Float32Array;ok=/* @__PURE__ */ new Uint8Array;lon0=0;m0=0;hts=/* @__PURE__ */ new Float32Array;hVer=-1;hAt=-1e9;hNext=-1;hPass=-1;dirty=!0;tex=null;cap=0;data=/* @__PURE__ */ new Float32Array;frame(e){if(this.set)return!!this.track(e);let t=this.items.length,n=e.graphics;if(!t)return!1;let r=e.world.groundVersion,i=t=>{let n=this.own[t];if(n===n){this.hts[t]=n;return}let r=e.world.heightAt(this.ll[2*t],this.ll[2*t+1]);this.hts[t]=Number.isNaN(r)?0:r};if(this.hts.length!==t){this.hts=new Float32Array(t);for(let e=0;e<t;e++)i(e);this.hVer=r,this.hAt=e.now,this.hNext=-1,this.dirty=!0}else this.hNext<0&&r!==this.hVer&&e.now-this.hAt>250&&(this.hNext=0,this.hPass=r);if(this.hNext>=0){let n=performance.now();for(;this.hNext<t&&(i(this.hNext++),!(!(this.hNext&255)&&performance.now()-n>2)););this.hNext>=t&&(this.hNext=-1,this.hVer=this.hPass,this.hAt=e.now,this.dirty=!0)}return this.dirty&&this.upload(n),!!this.tex}upload(e){let t=this.items.length,n=Math.max(1,Math.ceil(2*t/4096));(!this.tex||n>this.cap)&&(this.tex?.destroy(),this.cap=Math.max(n,this.cap*2||1),this.tex=e.texture({label:`placed places`,width:4096,height:this.cap,format:`rgba32f`,filter:`nearest`}),this.data=new Float32Array(this.cap*4096*4));let r=this.data;r.fill(0,0,n*4096*4);for(let e=0;e<t;e++){if(!this.ok[e])continue;let t=e*8,n=Math.max(-85.0511,Math.min(85.0511,this.ll[2*e+1]));r[t]=Y(this.ll[2*e]-this.lon0)*G,r[t+1]=q(n)-this.m0,r[t+2]=this.hts[e],r[t+3]=this.own[e]===this.own[e]?2:1,r[t+4]=this.turn[e],r[t+5]=this.liftOf[e]}this.tex.write(r.subarray(0,n*4096*4),{x:0,y:0,width:4096,height:n}),this.dirty=!1}track(e){let t=this.set?.track()??null;return t&&t.at===e.now?t:null}place(e,t=0){let n=e.camera;return[Y(this.lon0-n.lon)*G,this.m0-q(Math.max(-85.0511,Math.min(85.0511,n.lat))),e.world.exaggeration,t]}textures(e){return this.set?this.track(e)?.textures??[]:this.tex?[this.tex]:[]}blocks(e){let t=this.set?this.track(e):null;return t?[e.frame,t.block]:[e.frame]}get shown(){return this.host?this.host.shown:1}get busy(){return this.hNext>=0}lonLat(e,t){if(this.set){let n=this.set.log.at(e,t??Date.now(),!0);return n?[n.lon,n.lat]:null}return this.ok[e]?[this.ll[2*e],this.ll[2*e+1]]:null}};const Tc=(e,t)=>new wc(e,t);var Ec=class{names;size;cols;rows;images;decoded=/* @__PURE__ */ new Map;loading=null;textures=/* @__PURE__ */ new WeakMap;failed=[];constructor(e,t={}){if(this.names=Object.keys(e),!this.names.length)throw new K(`E-DATA-001`,`icons: give at least one picture`,{fix:`icons({ car: '/img/car.png' })`});this.images=e,this.size=Math.max(8,Math.min(256,Math.round(t.size??64))),this.cols=Math.min(this.names.length,Math.max(1,Math.floor(2048/this.size))),this.rows=Math.ceil(this.names.length/this.cols)}index(e){return this.names.indexOf(e)}load(e){return this.loading??=Promise.all(this.names.map(async t=>{let n=this.images[t];if(typeof n!=`string`)this.decoded.set(t,n);else try{let r=await e.get(n,{kind:`bytes`});if(!r)throw Error(`nothing at ${n}`);this.decoded.set(t,await createImageBitmap(new Blob([r])))}catch(e){this.failed.push(`${t}: ${e.message}`)}})).then(()=>{}),this.loading}get ready(){return this.decoded.size+this.failed.length>=this.names.length}texture(e){let t=this.textures.get(e);if(t)return t;if(!this.ready||typeof OffscreenCanvas>`u`)return null;let n=this.size,r=this.cols*n,i=this.rows*n,a=new OffscreenCanvas(r,i).getContext(`2d`,{willReadFrequently:!0});if(!a)return null;this.names.forEach((e,t)=>{let r=this.decoded.get(e);if(!r)return;let i=t%this.cols*n,o=Math.floor(t/this.cols)*n;if(a.save(),a.beginPath(),a.rect(i,o,n,n),a.clip(),typeof r==`function`)a.translate(i,o),r(a,n);else{let e=r.width||n,t=r.height||n,s=Math.min((n-2)/e,(n-2)/t);a.drawImage(r,i+(n-e*s)/2,o+(n-t*s)/2,e*s,t*s)}a.restore()});let o=a.getImageData(0,0,r,i).data,s=new Uint8Array(o.length);for(let e=0;e<o.length;e+=4){let t=o[e+3]/255;s[e]=Math.round(o[e]*t),s[e+1]=Math.round(o[e+1]*t),s[e+2]=Math.round(o[e+2]*t),s[e+3]=o[e+3]}let c=e.texture({label:`icons`,width:r,height:i,format:`rgba8`,filter:`linear`});return c.write(s),this.textures.set(e,c),c}};const Dc=(e,t)=>new Ec(e,t);var Oc=class extends D{kind=`point`;on;g=null;prog=null;pick=null;progKey=``;ok=!1;ubuf=null;u=/* @__PURE__ */ new Float32Array(16);quad=null;items=null;itemsCap=0;itemsSeen=-1;constructor(e){if(super(e.order===void 0?{...e,order:5.5}:e),!(e.icons instanceof Ec))throw new K(`E-DATA-001`,`icon '${this.id}' needs its atlas: icons({ … })`,{where:this.id,fix:`icon({ of, icons: icons({ car: '/img/car.png' }), icon: 'car' })`});let t=e.heading;this.on=new wc(e.of,{at:e.at,lift:e.lift,heading:typeof t==`function`?t:void 0})}async attach(e){await super.attach(e),this.g=e.graphics,this.on.attach(e,()=>{this.itemsSeen=-1,e.world.invalidate?.()}),this.g&&(this.ubuf=this.g.buffer(`uniform`,64,`${this.id} icon`),this.quad=this.g.buffer(`index`,Uint32Array.from([0,1,2,2,1,3]),`${this.id} quad`)),this.props.icons.load(e.loader).then(()=>e.world.invalidate?.())}setProps(e){return super.setProps(e),(`icon`in e||`size`in e||`color`in e)&&(this.itemsSeen=-1),`heading`in e&&(this.progKey=``),this}itemsUpload(){let e=this.on.items.length,t=Math.max(1,Math.ceil(2*e/4096)),n=this.props,r=n.icons,i=Ot(n.size);(!this.items||t>this.itemsCap)&&(this.items?.destroy(),this.itemsCap=Math.max(t,this.itemsCap*2||1),this.items=this.g.texture({label:`${this.id} items`,width:4096,height:this.itemsCap,format:`rgba32f`,filter:`nearest`}));let a=new Float32Array(t*4096*4);this.on.items.forEach((e,t)=>{let o=typeof n.icon==`function`?n.icon(e):n.icon,s=o?r.index(o):-1,c=L(typeof n.color==`function`?n.color(e):n.color??`#ffffff`);a.set([s,typeof i==`function`?Number(i(e))||0:i??24,0,0,c[0],c[1],c[2],c[3]],t*8)}),this.items.write(a,{x:0,y:0,width:4096,height:t}),this.itemsSeen=this.on.version}programs(e){let t=this.on.set?this.on.track(e)?.variant??null:null;if(this.on.set&&!t)return;let n=JSON.stringify(t);if(n===this.progKey)return;this.progKey=n,this.prog?.destroy(),this.pick?.destroy(),this.pick=null,this.ok=!1;let r=e=>e.map(e=>({name:e,kind:`2d`,sample:`unfilterable`})),i={label:`icon ${this.id}`,wgsl:Ss(t),glsl:zs(t),blocks:t?[`Frame`,`S`,`U`]:[`Frame`,`U`],textures:[...r(t?ss:[`places`]),...r([`items`]),{name:`atlas`,kind:`2d`,sample:`float`}],blend:`alpha`,depth:{write:!1,test:`always`}};this.desc=i,this.prog=this.g.program(i),this.broken=null,this.prog.ready.then(()=>{this.ok=!0,(this.attached?.world)?.invalidate?.()},e=>{this.broken=String(e?.message??e),this.attached?.world.error(e)})}desc=null;broken=null;frame(e){let t=e.graphics,n=this.props;if(!this.ubuf||!this.on.frame(e))return[];let r=n.icons.texture(t);if(!r||(this.programs(e),!this.prog||!this.ok))return[];this.itemsSeen!==this.on.version&&this.itemsUpload();let i=this.on.items.length;if(!i||!this.items)return[];let a=this.u,o=qt(n.size,`icon '${this.id}' size`);a.set(this.on.place(e,typeof n.lift==`number`?n.lift:0),0),a.set(o,4),a.set([(n.opacity??1)*this.shown*this.on.shown,n.icons.cols,n.icons.rows,+!!n.heading],8),a.set([this.pickNo,0,0,0],12),this.ubuf.write(a);let s=this.prog,c=this.desc;return[{program:s,blocks:[...this.on.blocks(e),this.ubuf],textures:[...this.on.textures(e),this.items,r],index:this.quad,count:6,instanceCount:i,pickProgram:n.pickable===!1?void 0:()=>this.pick??=t.program({...c,label:`${c.label} pick`,blend:`none`})}]}pickInfo(e){let t=this.on.items[e];if(t===void 0)return null;let n=this.on.set;if(n){let t=n.pickInfo(e);return t?{...t,element:this.id}:null}let r=t;return{element:this.id,id:r.id??e,props:r.props??{},d:t,index:e}}status(){let e=super.status();return this.props.icons.ready||e.pending.push(`${this.id}: its pictures loading`),this.on.busy&&e.pending.push(`${this.id}: reading the ground under its places`),this.broken?e.failed.push({what:`${this.id} program`,error:this.broken}):this.prog&&!this.ok&&e.pending.push(`${this.id}: its program building`),e.ready=!e.pending.length,e}wantsFrames(e){return this.on.busy?1/0:0}readouts(){return{count:()=>this.on.items.length}}detach(){this.on.detach(),this.prog?.destroy(),this.pick?.destroy(),this.prog=this.pick=null,this.progKey=``,this.ubuf?.destroy(),this.quad?.destroy(),this.items?.destroy(),this.ubuf=this.quad=null,this.items=null,this.itemsCap=0,this.itemsSeen=-1,super.detach()}};function kc(e){return new Oc(e)}const Ac=`${Be}
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,jc={vertex:`${Ac}
layout(location = 0) in vec4 ab;
layout(location = 1) in vec4 hf;
layout(location = 2) in vec2 nw;
out vec4 v_col;
out float v_across;
out float v_halfw;
out float v_along;
out float v_alongPx;
flat out float v_idx;
flat out vec4 v_state;
out vec3 v_world;
flat out float v_value;
vec2 screenOf(vec4 c) {
  return c.xy / c.w * frame.size.xy * 0.5;
}
vec2 perp(vec2 d) {
  return vec2(-d.y, d.x);
}
vec2 dirOf(vec2 a, vec2 b) {
  vec2 d = b - a;
  float l = length(d);
  if (l < 0.0001) {
    return vec2(1.0, 0.0);
  }
  return d / l;
}
vec3 unitOf(vec2 p) {
  float lat = 2.0 * atan(exp(clamp(frame.geo.x + p.y, -3.14, 3.14))) - PI * 0.5;
  float c = cos(lat);
  return vec3(c * cos(p.x), c * sin(p.x), sin(lat));
}
vec2 mercOf(vec3 v) {
  float lat = clamp(atan(v.z, length(v.xy)), -1.4844, 1.4844);
  return vec2(atan(v.y, v.x), log(tan(PI * 0.25 + lat * 0.5)) - frame.geo.x);
}
vec2 arcAt(vec4 ab, float w, float t) {
  vec2 p = mix(ab.xy, ab.zw, t);
  float k = smoothstep(0.0005, 0.002, w);
  if (k > 0.0) {
    vec3 va = unitOf(u.origin.xy + ab.xy);
    vec3 vb = unitOf(u.origin.xy + ab.zw);
    vec3 v = (sin((1.0 - t) * w) * va + sin(t * w) * vb) / sin(w);
    vec2 ga = mercOf(va);
    vec2 gb = mercOf(vb);
    gb.x = gb.x - 2.0 * PI * round((gb.x - ga.x) / (2.0 * PI));
    vec2 d = mercOf(v) - mix(ga, gb, t);
    d.x = d.x - 2.0 * PI * round(d.x / (2.0 * PI));
    p = p + d * k;
  }
  return p;
}
float arcH(vec4 hf, float t, float lift) {
  return mix(hf.x, hf.y, t) * u.origin.z + hf.z * 4.0 * t * (1.0 - t) + lift;
}
vec4 arcClip(vec4 ab, vec4 hf, float w, float t, float lift) {
  return frame.viewProj * vec4(placeAt(arcAt(ab, w, t), arcH(hf, t, lift)), 1.0);
}
void main() {
  float n = max(abs(nw.x), 1.0);
  float w = nw.y;
  float idx = hf.w;
  uint vid = uint(gl_VertexID);
  float at = float(vid / 2u);
  float k = min(at, n);
  float side = (vid & 1u) == 1u ? 1.0 : -1.0;
  float t = k / n;
  vec4 state = featAt(idx, 2);
  vec4 motion = vec4(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  float lift = u.origin.w + raiseOf(state) + motion.z;
  vec3 P = placeAt(arcAt(ab, w, t), arcH(hf, t, lift));
  vec4 c = frame.viewProj * vec4(liftToEye(P, u.b.z), 1.0);
  if (c.w <= 0.0) {
    gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
    return;
  }
  // a point past the arc's own: on its last point's middle, between that point's two vertices (nothing drawn)
  if (at > n) {
    gl_Position = clipOut(c);
    return;
  }
  vec2 s = screenOf(c);
  vec2 nA = vec2(0.0, 0.0);
  vec2 nB = vec2(0.0, 0.0);
  if (k > 0.0) {
    vec4 c0 = arcClip(ab, hf, w, (k - 1.0) / n, lift);
    if (c0.w > 0.0) {
      nA = perp(dirOf(screenOf(c0), s));
    }
  }
  if (k < n) {
    vec4 c1 = arcClip(ab, hf, w, (k + 1.0) / n, lift);
    if (c1.w > 0.0) {
      nB = perp(dirOf(s, screenOf(c1)));
    }
  }
  vec2 m = nA + nB;
  float ml = 1.0;
  if (dot(nA, nA) > 0.5 && dot(nB, nB) > 0.5) {
    m = normalize(m);
    ml = 1.0 / max(dot(m, nA), 0.5);
  } else if (dot(m, m) < 0.5) {
    m = vec2(0.0, 1.0);
  }
  vec4 nums = featAt(idx, 3);
  float wd = u.a.x;
  if (u.a.y > 0.5) {
    wd = wd * nums.x;
  }
  wd = sizePx(wd, u.size, P);
  wd = wd + u.hover2.y * state.x + u.sel2.y * state.y;
  wd = wd * frame.size.z * motion.w;
  float halfw = wd * 0.5 + 1.0;
  vec2 off = m * side * halfw * ml;
  gl_Position = clipOut(vec4(c.xy + off / (frame.size.xy * 0.5) * c.w, c.z, c.w));
  v_col = colourOf(idx, state);
  v_across = side * halfw * ml;
  v_halfw = halfw;
  v_along = nw.x < 0.0 ? 1.0 - t : t;
  v_alongPx = v_along * w * RE / max(mppAt(P) / max(frame.size.z, 0.001), 0.000001);
  v_idx = idx;
  v_state = state;
  v_world = P;
  v_value = nums.z;
}
`,fragment:`${Ac}
in vec4 v_col;
in float v_across;
in float v_halfw;
in float v_along;
in float v_alongPx;
flat in float v_idx;
flat in vec4 v_state;
in vec3 v_world;
flat in float v_value;
out vec4 outColor;
void main() {
  vec4 color = v_col;
  float alpha = color.a * clamp(v_halfw - abs(v_across), 0.0, 1.0) * clamp((v_halfw - 1.0) * 2.0, 0.0, 1.0);
  float along = v_along;
  float time = frame.eye.w;
  float value = v_value;
  vec3 position = v_world;
  float reveal = v_state.z;
  float mode = u.reveal.x;
  // dashes, flowing along at u.b.y px a second (0: still)
  float dashP = max(u.a.z + u.a.w, 0.0001);
  float dashAt = v_alongPx - time * u.b.y;
  if (u.a.z > 0.0 && mod(dashAt, dashP) > u.a.z) {
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
`},Mc=`${A}
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct I { @location(0) ab: vec4f, @location(1) hf: vec4f, @location(2) nw: vec2f, @builtin(vertex_index) vid: u32 }
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec4f,
  @location(1) across: f32,
  @location(2) halfw: f32,
  @location(3) along: f32,
  @location(4) alongPx: f32,
  @location(5) @interpolate(flat) idx: f32,
  @location(6) @interpolate(flat) state: vec4f,
  @location(7) world: vec3f,
  @location(8) @interpolate(flat) value: f32,
}
/*@functions*/
fn screenOf(c: vec4f) -> vec2f {
  return c.xy / c.w * frame.size.xy * 0.5;
}
fn perp(d: vec2f) -> vec2f {
  return vec2f(-d.y, d.x);
}
fn dirOf(a: vec2f, b: vec2f) -> vec2f {
  let d = b - a;
  let l = length(d);
  if (l < 0.0001) {
    return vec2f(1.0, 0.0);
  }
  return d / l;
}
// a place on the unit sphere (longitude from the view centre, latitude) and back (longitude, Mercator y)
fn unitOf(p: vec2f) -> vec3f {
  let lat = 2.0 * atan(exp(clamp(frame.geo.x + p.y, -3.14, 3.14))) - PI * 0.5;
  let c = cos(lat);
  return vec3f(c * cos(p.x), c * sin(p.x), sin(lat));
}
fn mercOf(v: vec3f) -> vec2f {
  let lat = clamp(atan2(v.z, length(v.xy)), -1.4844, 1.4844);
  return vec2f(atan2(v.y, v.x), log(tan(PI * 0.25 + lat * 0.5)) - frame.geo.x);
}
// the arc's place at t (rel. to the layer's origin): straight in Mercator, plus the great circle's departure from
// straight where the arc is long (from 3 km; the departure is taken between places made the same way, so the ends stay
// exact and float32's rounding cancels)
fn arcAt(ab: vec4f, w: f32, t: f32) -> vec2f {
  var p = mix(ab.xy, ab.zw, t);
  let k = smoothstep(0.0005, 0.002, w);
  if (k > 0.0) {
    let va = unitOf(u.origin.xy + ab.xy);
    let vb = unitOf(u.origin.xy + ab.zw);
    let v = (sin((1.0 - t) * w) * va + sin(t * w) * vb) / sin(w);
    let ga = mercOf(va);
    var gb = mercOf(vb);
    gb.x = gb.x - 2.0 * PI * round((gb.x - ga.x) / (2.0 * PI));
    var d = mercOf(v) - mix(ga, gb, t);
    d.x = d.x - 2.0 * PI * round(d.x / (2.0 * PI));
    p = p + d * k;
  }
  return p;
}
// its height at t (m): its ends' (× the exaggeration), the rise, the lift
fn arcH(hf: vec4f, t: f32, lift: f32) -> f32 {
  return mix(hf.x, hf.y, t) * u.origin.z + hf.z * 4.0 * t * (1.0 - t) + lift;
}
// a neighbour's place on the screen (only its direction is used: not moved towards the eye)
fn arcClip(ab: vec4f, hf: vec4f, w: f32, t: f32, lift: f32) -> vec4f {
  return frame.viewProj * vec4f(placeAt(arcAt(ab, w, t), arcH(hf, t, lift)), 1.0);
}
@vertex fn vs(i: I) -> O {
  let n = max(abs(i.nw.x), 1.0);
  let w = i.nw.y;
  let idx = i.hf.w;
  let at = f32(i.vid / 2u);
  let k = min(at, n);
  let side = select(-1.0, 1.0, (i.vid & 1u) == 1u);
  let t = k / n;
  let state = featAt(idx, 2);
  var motion = vec4f(0.0, 0.0, 0.0, 1.0);
  if (u.reveal.y > 0.5) {
    motion = featAt(idx, 5);
  }
  let lift = u.origin.w + raiseOf(state) + motion.z;
  let P = placeAt(arcAt(i.ab, w, t), arcH(i.hf, t, lift));
  let c = frame.viewProj * vec4f(liftToEye(P, u.b.z), 1.0);
  var o: O;
  if (c.w <= 0.0) {
    o.pos = vec4f(0.0, 0.0, 2.0, 1.0);
    return o;
  }
  // a point past the arc's own: on its last point's middle, between that point's two vertices (nothing drawn)
  if (at > n) {
    o.pos = clipOut(c);
    return o;
  }
  let s = screenOf(c);
  // the join: the segments before and after this point (t ± 1 / n), mitred
  var nA = vec2f(0.0, 0.0);
  var nB = vec2f(0.0, 0.0);
  if (k > 0.0) {
    let c0 = arcClip(i.ab, i.hf, w, (k - 1.0) / n, lift);
    if (c0.w > 0.0) {
      nA = perp(dirOf(screenOf(c0), s));
    }
  }
  if (k < n) {
    let c1 = arcClip(i.ab, i.hf, w, (k + 1.0) / n, lift);
    if (c1.w > 0.0) {
      nB = perp(dirOf(s, screenOf(c1)));
    }
  }
  var m = nA + nB;
  var ml = 1.0;
  if (dot(nA, nA) > 0.5 && dot(nB, nB) > 0.5) {
    m = normalize(m);
    ml = 1.0 / max(dot(m, nA), 0.5);
  } else if (dot(m, m) < 0.5) {
    m = vec2f(0.0, 1.0);
  }
  let nums = featAt(idx, 3);
  var wd = u.a.x;
  if (u.a.y > 0.5) {
    wd = wd * nums.x;
  }
  wd = sizePx(wd, u.size, P);
  wd = wd + u.hover2.y * state.x + u.sel2.y * state.y;
  wd = wd * frame.size.z * motion.w;
  let halfw = wd * 0.5 + 1.0;
  let off = m * side * halfw * ml;
  o.pos = clipOut(vec4f(c.xy + off / (frame.size.xy * 0.5) * c.w, c.z, c.w));
  o.col = colourOf(idx, state);
  o.across = side * halfw * ml;
  o.halfw = halfw;
  o.along = select(t, 1.0 - t, i.nw.x < 0.0);
  o.alongPx = o.along * w * RE / max(mppAt(P) / max(frame.size.z, 0.001), 0.000001);
  o.idx = idx;
  o.state = state;
  o.world = P;
  o.value = nums.z;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  var color = i.col;
  var alpha = color.a * clamp(i.halfw - abs(i.across), 0.0, 1.0) * clamp((i.halfw - 1.0) * 2.0, 0.0, 1.0);
  let along = i.along;
  let time = frame.eye.w;
  let value = i.value;
  let position = i.world;
  let reveal = i.state.z;
  let mode = u.reveal.x;
  // dashes, flowing along at u.b.y px a second (0: still)
  let dashP = max(u.a.z + u.a.w, 0.0001);
  let dashAt = i.alongPx - time * u.b.y;
  if (u.a.z > 0.0 && (dashAt - floor(dashAt / dashP) * dashP) > u.a.z) {
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
`;function Nc(e,t,n=64){let[r,i,a,o]=[e[0]*G,e[1]*G,t[0]*G,t[1]*G],s=[Math.cos(i)*Math.cos(r),Math.cos(i)*Math.sin(r),Math.sin(i)],c=[Math.cos(o)*Math.cos(a),Math.cos(o)*Math.sin(a),Math.sin(o)],l=Math.acos(Math.max(-1,Math.min(1,s[0]*c[0]+s[1]*c[1]+s[2]*c[2]))),u=[],d=e[0];for(let e=0;e<n;e++){let t=e/(n-1),r,i,a;if(l<1e-9)r=s[0],i=s[1],a=s[2];else{let e=Math.sin(l),n=Math.sin((1-t)*l)/e,o=Math.sin(t*l)/e;r=n*s[0]+o*c[0],i=n*s[1]+o*c[1],a=n*s[2]+o*c[2]}let o=Math.atan2(i,r)/G,f=Math.atan2(a,Math.hypot(r,i))/G;for(;o-d>180;)o-=360;for(;o-d<-180;)o+=360;d=o,u.push([o,f])}if(u[0]=[e[0],e[1]],n>1){let e=t[0];for(;e-u[n-2][0]>180;)e-=360;for(;e-u[n-2][0]<-180;)e+=360;u[n-1]=[e,t[1]]}return u}function Pc(e,t){if(typeof t==`number`)return Math.max(1,Math.min(512,Math.round(t)-1));let n=t?.every??25,r=t?.min??8,i=t?.max??64;return Math.max(1,Math.min(512,Math.min(i,Math.max(r,Math.ceil(e/n)))-1))}const Fc=(e,t)=>typeof t==`number`?e:e<=64?Math.ceil(e/8)*8:Math.ceil(e/64)*64,Ic=(e,t,n,r,i=`alpha`,a=!1)=>{let o=R({wgsl:Mc,glsl:jc},un.line,n,r,`layer '${t}'`);return new wn(e,{label:`arc ${t}`,wgsl:o.wgsl,glsl:o.glsl,blocks:[`Frame`,`L`,`Params`],textures:[{name:`feat`,kind:`2d`,sample:`unfilterable`}],instance:{stride:10,attributes:[{name:`ab`,location:0,size:4,offset:0},{name:`hf`,location:1,size:4,offset:4},{name:`nw`,location:2,size:2,offset:8}]},blend:i??`alpha`,depth:{write:!1,test:a?`always`:`less`}})},Lc=(e,t,n,r)=>e===void 0?r:typeof e==`function`?e(t,n):e;var Rc=class extends Ut{kind=`line`;mark=`line`;arcP;ends=/* @__PURE__ */ new Float64Array;buckets=/* @__PURE__ */ new Map;strip=null;stripN=0;stale=!0;full=!0;was=null;heightsAt=-1;constructor(e){if(typeof e.from!=`function`||typeof e.to!=`function`)throw new K(`E-DATA-001`,`arc '${e.id}' needs from: d => [lon, lat] and to: d => [lon, lat]`,{where:e.id,fix:`arc({ data, from: d => [d.props.olon, d.props.olat], to: d => [d.props.dlon, d.props.dlat] })`});super({...e,on:`absolute`}),this.arcP=e}drapeMark(){return`arc`}legacySits(){return this.props.over===void 0?void 0:this.props.over?`above`:`ground`}get order(){return this.props.order??(this.sits===`above`?4.5:void 0)}setProps(e){return super.setProps(e),this.arcP={...this.arcP,...e},[`from`,`to`,`height`,`fromHeight`,`toHeight`,`points`,`reveal`].some(t=>t in e)&&(this.stale=!0,this.full=!0),this}animating(e){return super.animating(e)||this.flowing()}wantsFrames(e){return super.animating(e)?1/0:this.flowing()?Sn(Math.abs(this.props.flow)):0}flowing(){let e=this.props;return!!e.dashes&&e.dashes[0]>0&&!!e.flow&&this.shown>0&&this.entries.length>0}programs(e){return[Ic(this.g,this.id,e,this.paramBlock,this.props.blend,this.sits===`above`)]}styleProps(){return[this.props.color,Ot(this.props.width),this.props.value]}sizeUnits(){return qt(this.props.width,`arc '${this.id}' width`)}style(e,t){let n=this.st;return this.colourInto(Lc(this.props.color,e.d,t,`#ffb000`)),n.number=Lc(Ot(this.props.width),e.d,t,2),n.value=Lc(this.props.value,e.d,t,0),n}read(e){let t=super.read(e),n=this.arcP,r=new Float64Array(t.length*4);for(let e=0;e<t.length;e++){let i=t[e],a=n.from(i.d),o=n.to(i.d);if(!(a&&o&&Number.isFinite(a[0])&&Number.isFinite(a[1])&&Number.isFinite(o[0])&&Number.isFinite(o[1]))){r.fill(NaN,e*4,e*4+4),i.geometry=null;continue}let s=a[0]+Y(o[0]-a[0]);r[e*4]=a[0],r[e*4+1]=a[1],r[e*4+2]=s,r[e*4+3]=o[1],i.geometry={type:`LineString`,coordinates:[[a[0],a[1]],[s,o[1]]]}}return this.ends=r,this.stale=!0,t}build(e){this.records(e),this.boxes()}records(e){this.stale=!1;let t=this.entries,n=this.arcP,r=e.s,i=this.origin,a=this.ends,o=e.graphics,s=this.full;this.full=!1;let c=typeof this.props.reveal==`object`?this.props.reveal.from:void 0,l=this.buckets,u=s?null:this.was,d=e.world.groundVersion,f=0,p=new Int32Array(t.length),m=new Int32Array(t.length).fill(-1),h=new Int32Array(t.length),g=new Float64Array(t.length),_=/* @__PURE__ */ new Map;for(let e=0;e<t.length;e++){let r=e*4;if(a[r]!==a[r])continue;let i=t[e],o=u?.byKey.get(i.key),s=o?u.pos.get(i.key)??-1:-1,c=s*4;if(o&&u&&o.d===i.d&&u.J[s]>=0&&u.E[c]===a[r]&&u.E[c+1]===a[r+1]&&u.E[c+2]===a[r+2]&&u.E[c+3]===a[r+3]){let t=u.N[s],n=u.J[s],r=l.get(t);if(r){p[e]=Math.abs(r.data[n*10+8]),m[e]=n,h[e]=t,_.set(t,(_.get(t)??0)+1),f++;continue}}let d=ae([a[r],a[r+1]],[a[r+2],a[r+3]]),v=Pc(d,n.points),y=Fc(v,n.points);p[e]=v,g[e]=d,_.set(y,(_.get(y)??0)+1)}let v=/* @__PURE__ */ new Map;for(let[e,t]of _){let n=l.get(e);v.set(e,{n:e,count:0,data:new Float32Array(t*10),of:new Int32Array(t),own:new Float32Array(t*2),buf:n?.buf??null,cap:n?.cap??0})}for(let[e,t]of l)_.has(e)||t.buf?.destroy();this.buckets=v;let y=new Int32Array(t.length),b=new Int32Array(t.length).fill(-1),x=(t,n)=>{let r=e.world.heightAt(t,n);return Number.isNaN(r)?0:r};for(let e=0;e<t.length;e++){let o=p[e];if(!o)continue;let s=e*4,u=t[e],d=m[e]>=0?h[e]:Fc(o,n.points),f=v.get(d),_=f.count++,S=_*10,C=f.data;if(m[e]>=0){let t=l.get(d),n=m[e];C.set(t.data.subarray(n*10,n*10+10),S),C[S+7]=u.index,f.own[_*2]=t.own[n*2],f.own[_*2+1]=t.own[n*2+1]}else{let t=g[e],l=i.rel(a[s],a[s+1]),d=i.rel(a[s+2],a[s+3]),p=Lc(n.fromHeight,u.d,r,0),m=Lc(n.toHeight,u.d,r,0),h=!!c&&(a[s+2]-c[0])**2+(a[s+3]-c[1])**2<(a[s]-c[0])**2+(a[s+1]-c[1])**2;C[S]=l[0],C[S+1]=l[1],C[S+2]=Un(d[0],l[0]),C[S+3]=d[1],C[S+4]=x(a[s],a[s+1])+p,C[S+5]=x(a[s+2],a[s+3])+m,C[S+6]=Lc(n.height,u.d,r,Math.min(15e5,t*.25)),C[S+7]=u.index,C[S+8]=h?-o:o,C[S+9]=t/J,f.own[_*2]=p,f.own[_*2+1]=m}f.of[_]=e,y[e]=d,b[e]=_}let S=/* @__PURE__ */ new Map;for(let e=0;e<t.length;e++)S.set(t[e].key,e);this.was={byKey:this.byKey,pos:S,E:a,N:y,J:b},(!f||this.heightsAt===d)&&(this.heightsAt=d);let C=0;for(let e of this.buckets.values()){if(C=Math.max(C,e.n),!e.buf||e.cap<e.count){let t=!!e.buf;e.buf?.destroy(),e.cap=t?Math.ceil(e.count*1.25):e.count,e.buf=o.buffer(`instance`,e.cap*10*4,`${this.id} arcs ×${e.n}`)}e.count&&e.buf.write(e.data)}(C>this.stripN||!this.strip)&&this.makeStrip(o,Math.max(C,8))}makeStrip(e,t){let n=new Uint32Array(t*6);for(let e=0;e<t;e++)n.set([2*e,2*e+1,2*e+2,2*e+2,2*e+1,2*e+3],e*6);this.strip?.destroy(),this.strip=e.buffer(`index`,n,`${this.id} strip`),this.stripN=t}ground(e){if(this.stale||this.heightsAt===e.world.groundVersion)return;this.heightsAt=e.world.groundVersion;let t=this.ends;for(let n of this.buckets.values()){let r=n.data,i=-1,a=-1;for(let o=0;o<n.count;o++){let s=n.of[o]*4,c=e.world.heightAt(t[s],t[s+1]),l=e.world.heightAt(t[s+2],t[s+3]),u=Math.fround((Number.isNaN(c)?0:c)+n.own[o*2]),d=Math.fround((Number.isNaN(l)?0:l)+n.own[o*2+1]);(u!==r[o*10+4]||d!==r[o*10+5])&&(r[o*10+4]=u,r[o*10+5]=d,i<0&&(i=o),a=o)}i>=0&&n.buf&&n.buf.write(r.subarray(i*10,(a+1)*10),i*10*4)}}uniforms(e){let t=this.props,n=t.dashes;this.L.set([1,1,n?.[0]??0,n?.[1]??0],28),this.L.set([t.glow??.5,n?t.flow??0:0,t.depthBias??2,0],32),this.L.set([0,0,0,0],36);let r=e?.world?.exaggeration;typeof r==`number`&&(this.L[2]=r)}draws(e){this.stale&&this.records(e);let t=this.progs[0];if(!t||!this.strip)return[];let n=[];for(let r of this.buckets.values())r.count&&r.buf&&n.push(this.call(t,{instances:r.buf,index:this.strip,count:r.n*6,instanceCount:r.count},e.frame));return n}freeGeometry(){for(let e of this.buckets.values())e.buf?.destroy();this.buckets.clear(),this.was=null,this.heightsAt=-1,this.strip?.destroy(),this.strip=null,this.stripN=0,this.stale=!0,this.full=!0}readouts(){return{count:()=>this.size,arcs:()=>{let e=0;for(let t of this.buckets.values())e+=t.count;return e},segments:()=>{let e=0;for(let t of this.buckets.values())for(let n=0;n<t.count;n++)e+=Math.abs(t.data[n*10+8]);return e}}}};const zc=e=>new Rc(e),Bc=e=>typeof e==`number`?e:e.getTime();var Vc=class extends Lo{tp;t0=0;t1=1;constructor(e){super(e),this.tp=e}timesOf(e){let t=this.tp.times?this.tp.times(e.d):e.d.props?.times;return t&&t.length?t:null}times=/* @__PURE__ */ new Float64Array;timeAt=/* @__PURE__ */ new Int32Array;timeN=/* @__PURE__ */ new Int32Array;startBuild(e){let t=this.entries,n=Array(t.length),r=0;for(let e=0;e<t.length;e++){let i=this.timesOf(t[e]);n[e]=i,r+=i?i.length:0}let i=new Float64Array(r),a=new Int32Array(this.bySlot.length),o=new Int32Array(this.bySlot.length),s=1/0,c=-1/0,l=0;for(let e=0;e<t.length;e++){let r=n[e];if(a[t[e].index]=l,o[t[e].index]=r?r.length:0,r)for(let e=0;e<r.length;e++){let t=Bc(r[e]);i[l++]=t,t<s&&(s=t),t>c&&(c=t)}}return this.times=i,this.timeAt=a,this.timeN=o,this.t0=Number.isFinite(s)?s:0,this.t1=Number.isFinite(c)&&c>s?c:this.t0+1,super.startBuild(e)}built2(){this.times=/* @__PURE__ */ new Float64Array,this.timeAt=/* @__PURE__ */ new Int32Array,this.timeN=/* @__PURE__ */ new Int32Array}alongOf(e,t){let n=this.timeN[e.index]??0;if(!n)return()=>0;let r=0;if(t){let n=z(e.geometry);for(let e=0;e<t;e++)r+=n[e]?.length??0}let i=this.times,a=this.timeAt[e.index],o=a+n-1,s=this.t0,c=this.t1-this.t0;return e=>{let t=Math.floor(e),n=e-t,l=i[Math.min(o,a+r+t)];return(l+(i[Math.min(o,a+r+t+1)]-l)*n-s)/c}}uniforms(e){super.uniforms(e);let t=this.tp.clock?Bc(this.tp.clock(e.s)):e.s.time.now.getTime(),n=this.t1-this.t0;this.L.set([(t-this.t0)/n,(this.tp.window??600)*1e3/n,1,this.tp.head??.6],36)}readouts(){return{...super.readouts(),from:()=>new Date(this.t0),to:()=>new Date(this.t1)}}selfMoving(){return!!this.tp.clock||this.attached?.world?.timeRunning===!0||super.selfMoving()}animating(e){return!!this.tp.clock||this.attached?.world?.timeRunning===!0||super.animating(e)}wantsFrames(e){return this.tp.clock||this.attached?.world?.timeRunning===!0?1/0:super.wantsFrames(e)}};const Hc=e=>new Vc(e);let Uc=0;const Wc=(e,t)=>({kind:`imagery`,id:e.id??`${t}-${++Uc}`,attribution:e.attribution??``,terms:{...Di,...e.terms},where:e.where,minZoom:e.minZoom??0,maxZoom:e.maxZoom??22,opacity:1,blend:`normal`,version:0}),Gc={xyz(e,t={}){if(typeof e!=`string`||!/\{z\}/.test(e))throw new K(`E-DATA-001`,`raster.xyz needs a URL with {z}, {x}, {y} (got ${JSON.stringify(e)})`,{fix:`raster.xyz('https://…/{z}/{x}/{y}.png')`});let n=Wc(t,`xyz`),r=t.maxZoom??18,i=(t,n,r)=>e.replace(`{z}`,String(t)).replace(`{x}`,String(n)).replace(`{y}`,String(r)).replace(`{-y}`,String(2**t-1-r));return{...n,maxZoom:r,urlFor:e=>{let t=Math.min(e.z,r),n=2**(e.z-t);return i(t,Math.floor(e.x/n),Math.floor(e.y/n))},async draw(e,t,a){if(t.z<n.minZoom)return!1;let o=Math.min(t.z,r),s=2**(t.z-o),c=Math.floor(t.x/s),l=Math.floor(t.y/s),u=await a.loader.get(i(o,c,l),{kind:`image`,priority:a.priority,prefetch:a.prefetch});if(!u)return!1;let d=u.width/s;return e.drawImage(u,(t.x-c*s)*d,(t.y-l*s)*d,d,d,0,0,256,256),!0}}},wms:Object.assign(qc,{capabilities:Jc,legend:Yc,info:Xc}),view(e){if(typeof e?.url!=`function`)throw new K(`E-DATA-001`,`raster.view needs url: (box, width, height) => string`,{fix:"raster.view({ url: ([w, s, e, n], width, height) => `https://…/export?bbox=${w},${s},${e},${n}&size=${width},${height}` })"});return{kind:`view`,id:e.id??`view-${++Uc}`,attribution:e.attribution??``,url:(t,n,r)=>Ct(e.url(t,n,r),e.proxy),maxSize:e.maxSize??2048,scale:e.scale??1,legend:e.legend,init:e.headers?{headers:e.headers}:void 0}},field(e){if(typeof e?.value!=`function`)throw new K(`E-DATA-001`,`raster.field needs value: (lon, lat) => number`,{fix:`raster.field({ value: (lon, lat) => …, ramp: 'inferno', domain: [0, 1] })`});let t=ve(e.ramp??`viridis`),[n,r]=e.domain??[0,1],i=Math.max(1,Math.min(64,e.cell??4)),a=e.alpha??1;return{...Wc(e,`field`),async draw(o,s){let c=!1;for(let l=0;l<256;l+=i)for(let u=0;u<256;u+=i){let[d,f]=I(s.z,s.x,s.y,(u+i/2)/256,(l+i/2)/256),p=e.value(d,f);p!==null&&Number.isFinite(p)&&(o.fillStyle=t(Math.max(0,Math.min(1,(p-n)/(r-n||1)))),o.globalAlpha=a,o.fillRect(u,l,i,i),c=!0)}return o.globalAlpha=1,c}}},image(e){let t=e?.bounds;if(!Array.isArray(t)||t.length!==4||!t.every(Number.isFinite)||!(t[2]>t[0])||!(t[3]>t[1]))throw new K(`E-DATA-001`,`raster.image needs bounds [w, s, e, n] (got ${JSON.stringify(t)})`,{fix:`raster.image({ url: '/heat.png', bounds: [145.09, -37.95, 145.16, -37.9] })`});if(!e.url&&!e.image)throw new K(`E-DATA-001`,`raster.image needs a url or an image`,{fix:`raster.image({ url: '/heat.png', bounds })`});return{kind:`image`,id:e.id??`image-${++Uc}`,attribution:e.attribution??``,bounds:[t[0],t[1],t[2],t[3]],url:e.url,image:e.image}},custom(e){if(typeof e?.draw!=`function`)throw new K(`E-DATA-001`,`raster.custom needs draw(g, tile)`,{fix:`raster.custom({ draw: (g, tile) => { …; return true } })`});return{...Wc(e,`custom`),draw:async(t,n,r)=>!!await e.draw(t,n,r)}}},Kc=20037508.342789244;function qc(e,t){if(!t?.layers)throw new K(`E-DATA-001`,`raster.wms needs { layers }`,{fix:`raster.wms('https://…/wms', { layers: 'VIIRS_Night' }) (raster.wms.capabilities(url) lists them)`});let n=t.version??`1.3.0`,r={SERVICE:`WMS`,REQUEST:`GetMap`,VERSION:n,LAYERS:t.layers,STYLES:t.styles??``,FORMAT:t.format??`image/png`,TRANSPARENT:String(t.transparent??!0),...t.params};if(t.single)return{...Gc.view({...t,id:t.id??`wms-${++Uc}`,legend:Yc(e,t.layers.split(`,`)[0],{style:t.styles,version:n,proxy:t.proxy}),url:([t,i,a,o],s,c)=>ct(e,{...r,...n.startsWith(`1.3`)?{CRS:`CRS:84`}:{SRS:`EPSG:4326`},BBOX:`${t},${i},${a},${o}`,WIDTH:s,HEIGHT:c})}),wms:{url:e,layers:t.layers,version:n}};let i=i=>{let a=2*Kc/2**i.z,o=-20037508.342789244+i.x*a,s=Kc-i.y*a;return Ct(ct(e,{...r,WIDTH:256,HEIGHT:256,[n.startsWith(`1.3`)?`CRS`:`SRS`]:`EPSG:3857`,BBOX:`${o},${s-a},${o+a},${s}`}),t.proxy)},a=t.headers?{headers:t.headers}:void 0;return{...Wc(t,`wms`),urlFor:i,async draw(e,t,n){let r=await n.loader.get(i(t),{kind:`image`,priority:n.priority,prefetch:n.prefetch,init:a});return r?(e.drawImage(r,0,0,256,256),!0):!1}}}function Jc(e,t){return Rn(e,t)}function Yc(e,t,n={}){return Ct(ct(e,{SERVICE:`WMS`,REQUEST:`GetLegendGraphic`,VERSION:n.version??`1.3.0`,FORMAT:n.format??`image/png`,LAYER:t,STYLE:n.style||void 0}),n.proxy)}async function Xc(e,t,n={}){let r=t.version??`1.3.0`,i=t.metres??(t.zoom===void 0?1:156543.03*Math.cos(t.lat*Math.PI/180)/2**t.zoom),a=i*50/110574,o=i*50/(111320*Math.max(.05,Math.cos(t.lat*Math.PI/180))),s=`${t.lon-o},${t.lat-a},${t.lon+o},${t.lat+a}`,c=ct(e,{SERVICE:`WMS`,REQUEST:`GetFeatureInfo`,VERSION:r,LAYERS:t.layers,QUERY_LAYERS:t.layers,STYLES:``,...r===`1.3.0`?{CRS:`CRS:84`,I:50,J:50}:{SRS:`EPSG:4326`,X:50,Y:50},BBOX:s,WIDTH:101,HEIGHT:101,INFO_FORMAT:t.format??`application/json`,FEATURE_COUNT:t.count??5}),l=await tt(c,n,`wms`),u;try{u=JSON.parse(l)}catch{throw new K(`E-DATA-002`,`the WMS did not answer GetFeatureInfo as GeoJSON (${l.slice(0,60).replace(/\s+/g,` `)}…)`,{where:e,fix:`ask with its own format name (format: 'application/geo+json' …): raster.wms.capabilities(url).infoFormats lists them`})}let d=u.features??[];return qe({type:`FeatureCollection`,features:d},{where:e}).map((e,t)=>({...e,layer:typeof d[t]?.id==`string`?d[t].id.replace(/\.[^.]*$/,``):null}))}var Zc=class extends D{kind=`raster`;constructor(e){if(super(e),!e.source||typeof e.source.draw!=`function`)throw new K(`E-DATA-001`,`raster '${this.id}' needs a source`,{where:this.id,fix:`raster({ source: raster.xyz('https://…/{z}/{x}/{y}.png') })`})}painted(){let e=Math.round(this.shown*4)/4;if(e<=0)return null;let t=this.props.source;return{...t,id:`${this.id}:${t.id}`,opacity:(this.props.opacity??1)*e,blend:this.props.blend??t.blend,when:void 0,draw:t.draw,urlFor:t.urlFor}}frame(){return[]}attribution(){return this.props.source.attribution?[this.props.source.attribution]:[]}get fades(){return!0}},Qc=class extends Ho{img;constructor(e){let t=e.source,n=t.kind===`image`?t.bounds:null;super({outline:0,pickable:!1,...e,data:n?[$c(n)]:[],color:`#ffffffff`}),this.img=t}drapeMark(){return`raster`}imaged(){return!0}loadErr=null;loading=!1;ctx=null;box=null;density=0;next=null;stillSince=-1;asks=0;retryAt=0;retryMs=2e3;due=!1;get picture(){return{box:this.box,asked:this.asks,loading:this.loading}}async attach(e){if(this.ctx=e,await super.attach(e),!e.graphics||this.img.kind!==`image`)return;let t=this.img;await this.load(async()=>t.image??(t.url?await e.loader.get(t.url,{kind:`image`,priority:`soon`}):null),t.url??`no url`,t.bounds)}async load(e,t,n){let r=this.ctx,i=r?.graphics;if(r&&i){this.loading=!0,this.asks++;try{let r=await e();if(!r)throw new K(`E-DATA-002`,`raster '${this.id}': its picture did not load (${t})`,{where:this.id,fix:`check the url, or give raster.image({ image }) a picture`});if(!this.attached)return;let a=typeof createImageBitmap==`function`?await createImageBitmap(r,{premultiplyAlpha:`premultiply`}):r;if(!this.attached)return;let o=i.texture({label:`${this.id} picture`,width:a.width,height:a.height,format:`rgba8`,filter:`linear`});o.writeImage(a,{premultiplied:!0}),this.loadErr=null,this.img.kind===`image`?(this.imageTex=o,this.box=n):(this.next?.tex.destroy(),this.next={tex:o,box:n},this.setProps({data:[$c(n)]})),this.retryMs=2e3}catch(e){this.loadErr=e instanceof Error?e:Error(String(e)),r.world.error(e),this.retryAt=r.world.now+this.retryMs,this.retryMs=Math.min(6e4,this.retryMs*2)}finally{this.loading=!1,r.world.invalidate?.()}}}frame(e){return this.img.kind===`view`&&(this.next&&=(this.imageTex?.destroy(),this.imageTex=this.next.tex,this.box=this.next.box,null),this.follow(e,this.img)),this.imageTex?super.frame(e):[]}follow(e,t){let n=e.camera,r=e.world.now;if(!this.ctx||n.moving||this.target<=0){this.stillSince=-1,this.due=!1;return}if(this.stillSince<0&&(this.stillSince=r),this.loading)return;let i=n.width,a=n.height,o=1/0,s=1/0,c=-1/0,l=-1/0,u=0;for(let e=0;e<=6;e++)for(let t=0;t<=4;t++){let r=n.lonLatAt(t/4*i,e/6*a);r&&(u++,o=Math.min(o,r[0]),c=Math.max(c,r[0]),s=Math.min(s,r[1]),l=Math.max(l,r[1]))}if(!u)return;let d=e.s.view,f=Math.max(.05,Math.cos(d.lat*Math.PI/180)),p=Math.max(i,a)*d.pixelSize*3/111320;o=Math.max(o,d.lon-p/f),c=Math.min(c,d.lon+p/f),s=Math.max(s,d.lat-p),l=Math.min(l,d.lat+p);let m=Math.max(1e-9,t.scale/d.pixelSize),h=(c-o)*.1,g=(l-s)*.1,_=[o-h,s-g,c+h,l+g],v=(_[2]-_[0])*111320*f,y=(_[3]-_[1])*110574,b=Math.min(t.maxSize,e.graphics.caps.maxTex||4096,this.ctx.profile?.quality===`lite`?1024:1/0),x=Math.min(m,b/Math.max(v,y));if(this.box){let e=this.box;if(o>=e[0]&&c<=e[2]&&s>=e[1]&&l<=e[3]&&x<=this.density*1.6){this.due=!1;return}}if(this.due=!0,this.box&&r-this.stillSince<250||r<this.retryAt)return;this.due=!1;let S=Math.max(16,Math.round(v*x)),C=Math.max(16,Math.round(y*x));this.density=x;let w=t.url(_,S,C),T=this.ctx.loader;this.load(()=>T.get(w,{kind:`image`,priority:`soon`,init:t.init}),w,_)}status(){let e=super.status();return this.loadErr?e.failed.push({what:this.id,error:this.loadErr.message}):(this.loading||this.next||this.due)&&(this.target>0||this.shown>0)&&(e.pending.push(this.due?`its picture due`:`its picture loading`),e.ready=!1),e}detach(){this.imageTex?.destroy(),this.imageTex=null,this.next?.tex.destroy(),this.next=null,this.box=null,this.ctx=null,super.detach()}attribution(){return this.img.attribution?[this.img.attribution]:[]}get legendImage(){return this.img.kind===`view`?this.img.legend??null:null}};const $c=([e,t,n,r])=>({id:`picture`,props:{},geometry:{type:`Polygon`,coordinates:[[[e,t],[n,t],[n,r],[e,r],[e,t]]]}});function el(e){let t=e.source?.kind;return t===`image`||t===`view`?new Qc(e):new Zc(e)}const tl=Object.assign(el,Gc);var nl=class{levels;o;regions=/* @__PURE__ */ new Map;kids=/* @__PURE__ */ new Map;boxes=/* @__PURE__ */ new Map;byLevel=/* @__PURE__ */ new Map;built=-1;constructor(e){if(!e?.levels?.length)throw new K(`E-DATA-001`,`a region tree needs levels, top first`,{fix:`regions.tree({ levels: ['lga', 'suburb'], sources: { … } })`});for(let t of e.levels)if(!e.sources?.[t])throw new K(`E-DATA-001`,`no source for level '${t}'`,{fix:`give sources: { ${t}: data.geojson(…) }`});this.o=e,this.levels=e.levels}key=(e,t)=>`${e}\u0000${t}`;async load(e){return await Promise.all(this.levels.map(t=>this.o.sources[t].start({loader:e,error:e=>{throw e}}))),this.build(),this}build(){let e=this.levels.map(e=>this.o.sources[e].version).reduce((e,t)=>e*31+t,7);e!==this.built&&(this.built=e,this.regions.clear(),this.kids.clear(),this.boxes.clear(),this.byLevel.clear(),this.levels.forEach((e,t)=>{let n=[];for(let r of this.o.sources[e].features()){let i=null;if(t>0){let n=this.levels[t-1],a=this.o.parentOf?.[e];if(a)i=a(r)??null;else{let e=H(r.geometry);e&&(i=this.find(e[0],e[1],n)?.id??null)}}let a={id:r.id,level:e,name:this.o.nameOf?this.o.nameOf(r,e):String(r.props.name??r.id),props:r.props,parent:i,feature:r};this.regions.set(this.key(e,r.id),a),n.push(a);let o=O(r.geometry);if(o&&this.boxes.set(this.key(e,r.id),o),i!==null){let n=this.key(this.levels[t-1],i);(this.kids.get(n)??this.kids.set(n,[]).get(n)).push(this.key(e,r.id))}}this.byLevel.set(e,n)}))}find(e,t,n){for(let r of this.byLevel.get(n)??[]){let i=this.boxes.get(this.key(n,r.id));if(!(i&&(e<i[0]||e>i[2]||t<i[1]||t>i[3]))&&dn(r.feature.geometry,[e,t]))return r}return null}at(e,t,n=this.levels[this.levels.length-1]){return this.build(),this.find(e,t,n)}get(e,t){return this.build(),this.regions.get(this.key(e,t))??null}parent(e){let t=this.levels.indexOf(e.level);return t>0&&e.parent!==null?this.get(this.levels[t-1],e.parent):null}children(e){return this.build(),(this.kids.get(this.key(e.level,e.id))??[]).map(e=>this.regions.get(e))}path(e){let t=[e],n=this.parent(e);for(;n;)t.unshift(n),n=this.parent(n);return t}regionsOf(e){return this.build(),this.byLevel.get(e)??[]}level(e){let t=this.o.sources[e];if(!t)throw new K(`E-DATA-001`,`no level '${e}'`,{fix:`the levels: ${this.levels.join(`, `)}`});return t}rollup(e,t){let n=t.from??this.levels[this.levels.length-1],r=this.levels.indexOf(n),i=this.levels.indexOf(t.to);if(r<0||i<0||i>r)throw new K(`E-DATA-001`,`cannot roll '${n}' up to '${t.to}'`,{fix:`roll a lower level up to a higher one of ${this.levels.join(` → `)}`});let a=new rl([e,...this.levels.map(e=>this.o.sources[e])]),o=()=>{this.build();let r=/* @__PURE__ */ new Map;for(let i of e.features()){let e=this.get(n,i.props[t.on]);for(;e&&e.level!==t.to;)e=this.parent(e);if(!e)continue;let a=String(e.id);(r.get(a)??r.set(a,{region:e,rows:[]}).get(a)).rows.push(i.props)}a.set([...r.values()].map(({region:e,rows:n})=>{let r={id:e.id,name:e.name};for(let e of t.sum??[])r[e]=n.reduce((t,n)=>t+(Number(n[e])||0),0);for(let e of t.mean??[]){let t=n.map(t=>Number(t[e])).filter(Number.isFinite);r[e]=t.length?t.reduce((e,t)=>e+t,0)/t.length:null}return t.count&&(r.count=n.length),t.fn?{...r,...t.fn(n,e)}:r}))};e.watch(o);for(let e of this.levels)this.o.sources[e].watch(o);return o(),a}},rl=class extends at{deps;constructor(e){super([],{id:`id`}),this.deps=e}async load(e){await Promise.all(this.deps.map(t=>t.start(e)))}};const il={tree:e=>new nl(e)},al=`${mt}
layout(std140) uniform U { vec4 misc; vec4 pt; vec4 size; } u;
layout(std140) uniform Params { /*@params*/ } params;
/*@textures*/
vec3 lambert(vec3 n) {
  return frame.sunColor.w > 0.5 ? frame.ambient.rgb + frame.sunColor.rgb * max(dot(n, frame.sun.xyz), 0.0) : vec3(1.0);
}
/*@functions*/
`,ol={vertex:`${al}
layout(location = 0) in vec3 a_position;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;
layout(location = 3) in vec4 a_color;
layout(location = 4) in vec4 i_at;
/*@attributes*/
out vec2 v_uv;
out vec3 v_normal;
out vec3 v_world;
out vec4 v_color;
flat out float v_inst;
out vec2 v_corner;
/*@varyings*/
vec2 cornerOf(int v) {
  float x = (v == 1 || v == 4 || v == 5) ? 1.0 : -1.0;
  float y = (v == 2 || v == 3 || v == 5) ? 1.0 : -1.0;
  return vec2(x, y);
}
void main() {
  vec3 position = a_position;
  vec3 normal = a_normal;
  vec2 uv = a_uv;
  vec4 color = a_color;
  float time = frame.eye.w;
  vec3 eye = frame.eye.xyz;
  vec3 origin = i_at.xyz;
  /*@inputs*/
  /*@vertex*/
  float keep = u.size.w > 0.5 ? sizeKeep(u.size.x, vec4(1.0, u.size.y, u.size.z, 0.0), i_at.xyz) : 1.0;
  vec3 world = i_at.xyz + position * keep;
  vec4 clip = frame.viewProj * vec4(world, 1.0);
  vec2 corner = vec2(0.0);
  if (u.pt.y > 0.5) {
    corner = cornerOf(gl_VertexID % 6);
    clip = vec4(clip.xy + corner * u.pt.x * frame.size.z / frame.size.xy * clip.w, clip.z, clip.w);
  }
  gl_Position = clipOut(vec4(clip.xy, clip.z - u.misc.x * clip.w, clip.w));
  v_uv = uv;
  v_normal = normal;
  v_world = world;
  v_color = color;
  v_inst = float(gl_InstanceID);
  v_corner = corner;
  /*@outputs*/
}
`,fragment:`${al}
in vec2 v_uv;
in vec3 v_normal;
in vec3 v_world;
in vec4 v_color;
flat in float v_inst;
in vec2 v_corner;
/*@varyings*/
out vec4 outColor;
void main() {
  if (frame.pick.y > 0.5) {
    outColor = packDepth(gl_FragCoord.z);
    return;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.misc.y, v_inst);
    return;
  }
  vec2 uv = v_uv;
  vec3 normal = normalize(v_normal);
  vec3 position = v_world;
  float time = frame.eye.w;
  vec2 corner = v_corner;
  vec4 color = v_color;
  /*@finputs*/
  /*@fragment*/
  float alpha = color.a * u.misc.z * u.misc.w;
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(color.rgb, 0.0), alpha);
}
`},sl=`${nn}
// misc: (depth bias, the layer's pick number, opacity, reveal) · pt: (a point's size in CSS px, 1 when the vertices are
// points: each drawn as six (a square facing the screen), 0, 0) · size: a copy's range on screen (its geometry's size
// across m, the least and the most CSS px, 1 when set: core › render › drawing › size)
struct U { misc: vec4f, pt: vec4f, size: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
/*@textures*/
struct I { @location(0) a_position: vec3f, @location(1) a_normal: vec3f, @location(2) a_uv: vec2f, @location(3) a_color: vec4f, @location(4) i_at: vec4f, /*@attributes*/ }
struct O { @builtin(position) pos: vec4f, @location(0) v_uv: vec2f, @location(1) v_normal: vec3f, @location(2) v_world: vec3f, @location(3) v_color: vec4f, @location(4) @interpolate(flat) v_inst: f32, @location(5) v_corner: vec2f, /*@varyings*/ }
// the sky's light on a surface facing n (its ambient and the sun, no shadows): multiply a colour by it
fn lambert(n: vec3f) -> vec3f {
  return select(vec3f(1.0), frame.ambient.rgb + frame.sunColor.rgb * max(dot(n, frame.sun.xyz), 0.0), frame.sunColor.w > 0.5);
}
/*@functions*/
fn cornerOf(v: i32) -> vec2f {
  let x = select(-1.0, 1.0, v == 1 || v == 4 || v == 5);
  let y = select(-1.0, 1.0, v == 2 || v == 3 || v == 5);
  return vec2f(x, y);
}
@vertex fn vs(i: I, @builtin(instance_index) inst: u32, @builtin(vertex_index) vid: u32) -> O {
  var position = i.a_position;
  var normal = i.a_normal;
  let uv = i.a_uv;
  var color = i.a_color;
  let time = frame.eye.w;
  let eye = frame.eye.xyz;
  let origin = i.i_at.xyz;
  /*@inputs*/
  /*@vertex*/
  // a copy kept within its range of CSS px (round its place)
  let keep = select(1.0, sizeKeep(u.size.x, vec4f(1.0, u.size.y, u.size.z, 0.0), i.i_at.xyz), u.size.w > 0.5);
  let world = i.i_at.xyz + position * keep;
  var clip = frame.viewProj * vec4f(world, 1.0);
  var corner = vec2f(0.0);
  if (u.pt.y > 0.5) {
    corner = cornerOf(i32(vid % 6u));
    clip = vec4f(clip.xy + corner * u.pt.x * frame.size.z / frame.size.xy * clip.w, clip.z, clip.w);
  }
  var o: O;
  o.pos = clipOut(vec4f(clip.xy, clip.z - u.misc.x * clip.w, clip.w));
  o.v_uv = uv;
  o.v_normal = normal;
  o.v_world = world;
  o.v_color = color;
  o.v_inst = f32(inst);
  o.v_corner = corner;
  /*@outputs*/
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  if (frame.pick.y > 0.5) {
    return packDepth(i.pos.z);
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.misc.y, i.v_inst);
  }
  let uv = i.v_uv;
  let normal = normalize(i.v_normal);
  let position = i.v_world;
  let time = frame.eye.w;
  let corner = i.v_corner;
  var color = i.v_color;
  /*@finputs*/
  /*@fragment*/
  let alpha = color.a * u.misc.z * u.misc.w;
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(color.rgb, 0.0), alpha);
}
`,cl=`${mt}
layout(std140) uniform U { vec4 misc; vec4 size; } u;
layout(std140) uniform Params { /*@params*/ } params;
/*@functions*/
`,ll={vertex:`${cl}
layout(location = 0) in vec4 at;
/*@attributes*/
out vec2 v_corner;
/*@varyings*/
vec3 towardEye(vec3 p, float discPx) {
  vec3 toEye = frame.eye.xyz - p;
  float d = length(toEye);
  float mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(discPx * frame.size.z * 0.5 * mpp, d * 0.5);
}
vec2 cornerOf(int v) {
  float x = (v == 1 || v == 4 || v == 5) ? 1.0 : -1.0;
  float y = (v == 2 || v == 3 || v == 5) ? 1.0 : -1.0;
  return vec2(x, y);
}
void main() {
  vec2 corner = cornerOf(gl_VertexID);
  vec3 position = at.xyz;
  float size = at.w;
  float time = frame.eye.w;
  if (u.misc.y > 0.5) {
    size = u.size.x > 0.5 ? size * sizeKeep(size, u.size, position) : sizePx(size, u.size, position) * cssMpp(position);
  } else {
    size = sizePx(size, u.size, position);
  }
  /*@inputs*/
  /*@vertex*/
  vec4 clip = frame.viewProj * vec4(position, 1.0);
  if (u.misc.y > 0.5) {
    clip = frame.viewProj * vec4(position + vec3(corner.x, 0.0, -corner.y) * size * 0.5, 1.0);
  } else {
    clip = frame.viewProj * vec4(towardEye(position, size), 1.0);
    clip = vec4(clip.xy + corner * size * frame.size.z / frame.size.xy * clip.w, clip.z, clip.w);
  }
  gl_Position = clipOut(vec4(clip.xy, clip.z - u.misc.x * clip.w, clip.w));
  v_corner = corner;
  /*@outputs*/
}
`,fragment:`${cl}
in vec2 v_corner;
/*@varyings*/
out vec4 outColor;
void main() {
  vec2 corner = v_corner;
  float time = frame.eye.w;
  vec4 color = vec4(1.0);
  /*@finputs*/
  /*@fragment*/
  float alpha = color.a * u.misc.z * u.misc.w;
  if (alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(color.rgb, 0.0), alpha);
}
`},ul=`${nn}
// misc: (depth bias, shape: 0 screen / 1 ground, opacity, reveal) · size: how an item's size is drawn (units: 0 CSS px,
// 1 metres; the least and the most CSS px: core › render › drawing › size)
struct U { misc: vec4f, size: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
struct I { @builtin(vertex_index) vid: u32, @location(0) at: vec4f, /*@attributes*/ }
struct O { @builtin(position) pos: vec4f, @location(0) corner: vec2f, /*@varyings*/ }
/*@functions*/
fn towardEye(p: vec3f, discPx: f32) -> vec3f {
  let toEye = frame.eye.xyz - p;
  let d = length(toEye);
  let mpp = 2.0 * d * frame.misc.z / frame.size.y;
  return p + toEye / max(d, 0.001) * min(discPx * frame.size.z * 0.5 * mpp, d * 0.5);
}
fn cornerOf(v: i32) -> vec2f {
  let x = select(-1.0, 1.0, v == 1 || v == 4 || v == 5);
  let y = select(-1.0, 1.0, v == 2 || v == 3 || v == 5);
  return vec2f(x, y);
}
@vertex fn vs(i: I) -> O {
  let corner = cornerOf(i32(i.vid));
  var position = i.at.xyz;
  var size = i.at.w;
  let time = frame.eye.w;
  // its size by the layer's rule: facing the screen, CSS px; on the ground, metres (CSS px there as metres)
  if (u.misc.y > 0.5) {
    size = select(sizePx(size, u.size, position) * cssMpp(position), size * sizeKeep(size, u.size, position), u.size.x > 0.5);
  } else {
    size = sizePx(size, u.size, position);
  }
  /*@inputs*/
  /*@vertex*/
  var clip = frame.viewProj * vec4f(position, 1.0);
  if (u.misc.y > 0.5) {
    clip = frame.viewProj * vec4f(position + vec3f(corner.x, 0.0, -corner.y) * size * 0.5, 1.0);
  } else {
    clip = frame.viewProj * vec4f(towardEye(position, size), 1.0);
    clip = vec4f(clip.xy + corner * size * frame.size.z / frame.size.xy * clip.w, clip.z, clip.w);
  }
  var o: O;
  o.pos = clipOut(vec4f(clip.xy, clip.z - u.misc.x * clip.w, clip.w));
  o.corner = corner;
  /*@outputs*/
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let corner = i.corner;
  let time = frame.eye.w;
  var color = vec4f(1.0);
  /*@finputs*/
  /*@fragment*/
  let alpha = color.a * u.misc.z * u.misc.w;
  if (alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(color.rgb, 0.0), alpha);
}
`;var dl=class extends D{kind=`custom`;prog=null;inst=null;ubuf=null;pbuf=null;attrs=[];animating(){let e=this.props;return/\btime\b|frame\.eye\.w/.test(`${e.vertex??``} ${e.fragment??``}`)}stride=4;cap=0;buf=/* @__PURE__ */ new Float32Array;u=/* @__PURE__ */ new Float32Array(8);n=0;constructor(e){if(super({...e,params:{...e.uniforms??{},...e.params??{}}}),!Array.isArray(e.data))throw new K(`E-DATA-001`,`custom '${this.id}': data must be an array`,{where:this.id,fix:`pass data: [...]`});if(typeof e.at!=`function`)throw new K(`E-DATA-001`,`custom '${this.id}': at must be a function d => [lon, lat, h?]`,{where:this.id,fix:`at: d => d.at`});this.paramBlock??=Ye({});let t=e.data[0];for(let[n,r]of Object.entries(e.attributes??{})){let e=t===void 0?0:r(t),i=Array.isArray(e)?e.length:1;if(i<1||i>4)throw new K(`E-SHADER-001`,`attribute '${n}' gives ${i} numbers`,{where:this.id,fix:`give a number or 2 to 4 numbers`});this.attrs.push({name:n,size:i,get:r})}this.stride=4+this.attrs.reduce((e,t)=>e+t.size,0)}code(){let e=this.props;return{attributes:Object.fromEntries(this.attrs.map(e=>[e.name,e.size===1?`f32`:`vec${e.size}f`])),varyings:e.varyings??{},vertex:e.vertex,fragment:e.fragment,functions:e.functions}}async attach(e){await super.attach(e),e.graphics&&(this.ubuf=e.graphics.buffer(`uniform`,32,`${this.id} misc`),this.pbuf=e.graphics.buffer(`uniform`,this.paramBlock.size,`${this.id} params`),this.prog=this.build(this.code()),await this.prog.ready)}build(e){let t=Ke({wgsl:ul,glsl:ll},e,this.paramBlock,`layer '${this.id}'`),n=4,r=[{name:`at`,location:0,size:4,offset:0},...t.attributes.map((e,t)=>{let r={name:`a_${e.name}`,location:t+1,size:e.size,offset:n};return n+=e.size,r})];return this.attached.graphics.program({label:`custom ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`U`,`Params`],instance:{stride:this.stride,attributes:r},blend:this.props.blend??`alpha`,depth:{write:!1,test:`less`}})}async setShader(e){let t={...this.props,...e};if(!this.attached?.graphics){this.props=t;return}let n=this.props;this.props=t;let r;try{r=this.build(this.code())}catch(e){throw this.props=n,e}try{await r.ready}catch(e){throw r.destroy(),this.props=n,e}this.prog?.destroy(),this.prog=r}readouts(){return{count:()=>this.props.data.length,drawn:()=>this.n}}frame(e){if(!this.prog||!this.ubuf)return[];let t=this.props,n=e.s,r=e.camera,i=e.world.exaggeration,a=t.data.length,o=this.stride;(!this.inst||this.cap<a)&&(this.inst?.destroy(),this.cap=Math.max(64,a*2),this.inst=e.graphics.buffer(`instance`,this.cap*o*4,`${this.id} instances`),this.buf=new Float32Array(this.cap*o));let s=0,c=Ot(t.size);for(let a of t.data){let l=t.at(a);if(!r.facing(l[0],l[1]))continue;let u=l[2];if(u===void 0){let t=e.world.heightAt(l[0],l[1]);u=Number.isNaN(t)?0:t}let d=we(r,l[0],l[1],u*i),f=s*o;this.buf.set([d[0],d[1],d[2],typeof c==`function`?c(a,n):c??12],f);let p=f+4;for(let e of this.attrs){let t=e.get(a);Array.isArray(t)?this.buf.set(t,p):this.buf[p]=t,p+=e.size}s++}return this.n=s,s?(this.inst.write(this.buf.subarray(0,s*o)),this.u.set([t.bias??2e-5,+(t.shape===`ground`),t.opacity??1,this.shown]),this.u.set(t.shape===`ground`&&(t.size===void 0||typeof t.size!=`object`)?[1,0,1e6,0]:qt(t.size,`custom '${this.id}' size`),4),this.ubuf.write(this.u),this.paramsDirty&&=(this.pbuf.write(this.paramBlock.pack(this.paramValues)),!1),[{program:this.prog,blocks:[e.frame,this.ubuf,this.pbuf],instances:this.inst,count:6,instanceCount:s}]):[]}detach(){for(let e of[this.prog,this.inst,this.ubuf,this.pbuf])e?.destroy();this.prog=null,this.inst=this.ubuf=this.pbuf=null,super.detach()}};function fl(e){return`geometry`in e&&e.geometry?new ml(e):new dl(e)}const pl=e=>typeof e==`object`&&!!e&&!Array.isArray(e)&&`image`in e;var ml=class extends D{kind=`custom`;prog=null;vbuf=null;ibuf=null;inst=null;ubuf=null;pbuf=null;texs=[];texNames=[];stride=12;count=0;istride=4;cap=0;buf=/* @__PURE__ */ new Float32Array;u=/* @__PURE__ */ new Float32Array(12);n=0;across=0;vattrs=[];iattrs=[];drawnOrder=[];constructor(e){let t=Object.fromEntries(Object.entries(e.uniforms??{}).filter(([,e])=>!pl(e)));super({...e,params:{...t,...e.params??{}}});let n=e.geometry;if(!n||!n.positions||n.positions.length<3)throw new K(`E-DATA-001`,`custom '${this.id}': geometry.positions needs x, y, z per vertex`,{where:this.id,fix:`geometry: { positions: [0, 0, 0, 1, 0, 0, 0, 1, 0] }`});if(e.data&&typeof e.at!=`function`)throw new K(`E-DATA-001`,`custom '${this.id}': with data, at must be a function d => [lon, lat, h?]`,{where:this.id,fix:`at: d => d.at`});this.paramBlock??=Ye({}),this.texNames=Object.entries(e.uniforms??{}).filter(([,e])=>pl(e)).map(([e])=>e);let r=e.data?.[0];for(let[t,n]of Object.entries(e.attributes??{})){let e=r===void 0?0:n(r),i=Array.isArray(e)?e.length:1;this.iattrs.push({name:t,size:i,get:n})}this.istride=4+this.iattrs.reduce((e,t)=>e+t.size,0),this.vattrs=Object.entries(n.attributes??{}).map(([e,t])=>({name:e,size:t.size})),this.stride=12+this.vattrs.reduce((e,t)=>e+t.size,0)}vertices(){let e=this.props.geometry,t=Math.floor(e.positions.length/3),n=this.stride,r=this.props.primitive===`points`,i=e.colors?e.colors.length>=t*4?4:3:0,a=(t,n,r)=>{n[r]=e.positions[t*3],n[r+1]=e.positions[t*3+1],n[r+2]=e.positions[t*3+2],n[r+3]=e.normals?e.normals[t*3]:0,n[r+4]=e.normals?e.normals[t*3+1]:1,n[r+5]=e.normals?e.normals[t*3+2]:0,n[r+6]=e.uvs?e.uvs[t*2]:0,n[r+7]=e.uvs?e.uvs[t*2+1]:0,n[r+8]=i?e.colors[t*i]:1,n[r+9]=i?e.colors[t*i+1]:1,n[r+10]=i?e.colors[t*i+2]:1,n[r+11]=i===4?e.colors[t*4+3]:1;let a=r+12;for(let r of this.vattrs){let i=e.attributes[r.name].data;for(let e=0;e<r.size;e++)n[a++]=i[t*r.size+e]??0}};if(r){let r=e.index?Array.from(e.index):Array.from({length:t},(e,t)=>t),i=new Float32Array(r.length*6*n);return r.forEach((e,t)=>{for(let r=0;r<6;r++)a(e,i,(t*6+r)*n)}),{data:i,index:null}}let o=new Float32Array(t*n);for(let e=0;e<t;e++)a(e,o,e*n);return{data:o,index:e.index?Uint32Array.from(e.index):null}}async attach(e){await super.attach(e);let t=e.graphics;if(!t)return;let n=this.props,r=this.vertices();this.vbuf=t.buffer(`vertex`,r.data,`${this.id} vertices`),this.ibuf=r.index?t.buffer(`index`,r.index,`${this.id} index`):null,this.count=r.index?r.index.length:r.data.length/this.stride,this.ubuf=t.buffer(`uniform`,48,`${this.id} misc`);{let e=n.geometry.positions,t=0;for(let n=0;n+2<e.length;n+=3)t=Math.max(t,Math.hypot(e[n],e[n+1],e[n+2]));this.across=2*t}this.pbuf=t.buffer(`uniform`,this.paramBlock.size,`${this.id} params`);for(let e of this.texNames){let r=n.uniforms[e].image;typeof HTMLImageElement<`u`&&r instanceof HTMLImageElement&&(await r.decode().catch(()=>{}),r=await createImageBitmap(r));let i=r.width,a=r.height,o=t.texture({label:`${this.id} ${e}`,width:Math.max(1,i),height:Math.max(1,a),format:`rgba8`,filter:`linear`});o.writeImage(r),this.texs.push(o)}this.prog=this.build(),await this.prog.ready}build(){let e=this.props,t=Ue({wgsl:sl,glsl:ol},{vertexAttributes:Object.fromEntries(this.vattrs.map(e=>[e.name,e.size===1?`f32`:`vec${e.size}f`])),instanceAttributes:Object.fromEntries(this.iattrs.map(e=>[e.name,e.size===1?`f32`:`vec${e.size}f`])),varyings:e.varyings??{},textures:this.texNames,vertex:e.vertex,fragment:e.fragment,functions:e.functions},this.paramBlock,`layer '${this.id}'`),n=12,r=[{name:`a_position`,location:0,size:3,offset:0},{name:`a_normal`,location:1,size:3,offset:3},{name:`a_uv`,location:2,size:2,offset:6},{name:`a_color`,location:3,size:4,offset:8},...t.vertexAttrs.map((e,t)=>{let r={name:`a_${e.name}`,location:5+t,size:e.size,offset:n};return n+=e.size,r})],i=4,a=[{name:`i_at`,location:4,size:4,offset:0},...t.instanceAttrs.map((e,n)=>{let r={name:`a_${e.name}`,location:5+t.vertexAttrs.length+n,size:e.size,offset:i};return i+=e.size,r})],o=e.depth??{},s=e.side??`double`;return this.attached.graphics.program({label:`custom ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`U`,`Params`],textures:this.texNames.map(e=>({name:`t_${e}`,kind:`2d`,sample:`float`})),vertex:{stride:this.stride,attributes:r},instance:{stride:this.istride,attributes:a},blend:e.blend??`none`,depth:{test:o.test===!1?`always`:`less`,write:o.write??(e.blend??`none`)===`none`},cull:s===`double`?`none`:s===`front`?`back`:`front`,topology:e.primitive===`lines`?`lines`:`triangles`})}readouts(){return{copies:()=>this.n}}frame(e){if(!this.prog||!this.ubuf||!this.vbuf)return[];let t=this.props,n=e.camera,r=e.world.exaggeration,i=this.istride,a=t.data??[null],o=a.length;(!this.inst||this.cap<o)&&(this.inst?.destroy(),this.cap=Math.max(16,o*2),this.inst=e.graphics.buffer(`instance`,this.cap*i*4,`${this.id} copies`),this.buf=new Float32Array(this.cap*i));let s=[];for(let i=0;i<o;i++){let o=a[i],c=typeof t.at==`function`?t.at(o):t.at??[n.lon,n.lat];if(!n.facing(c[0],c[1]))continue;let l=c[2];if(l===void 0){let t=e.world.heightAt(c[0],c[1]);l=Number.isNaN(t)?0:t}let u=we(n,c[0],c[1],l*r),d=n.eye;s.push({k:i,P:u,d2:(u[0]-d[0])**2+(u[1]-d[1])**2+(u[2]-d[2])**2})}if((t.blend??`none`)!==`none`&&s.sort((e,t)=>t.d2-e.d2),this.drawnOrder=s.map(e=>e.k),s.forEach((e,t)=>{let n=t*i;this.buf.set([e.P[0],e.P[1],e.P[2],0],n);let r=n+4,o=a[e.k];for(let e of this.iattrs){let t=e.get(o);Array.isArray(t)?this.buf.set(t,r):this.buf[r]=t,r+=e.size}}),this.n=s.length,!this.n)return[];this.inst.write(this.buf.subarray(0,this.n*i)),this.u.set([t.bias??0,this.pickNo,t.opacity??1,this.shown,t.pointSize??6,+(t.primitive===`points`),0,0]);{let e=bt(t.size,`custom '${this.id}' size`);this.u.set([this.across,e[1],e[2],+!!t.size],8)}this.ubuf.write(this.u),this.paramsDirty&&=(this.pbuf.write(this.paramBlock.pack(this.paramValues)),!1);let c=this.prog;return[{program:c,blocks:[e.frame,this.ubuf,this.pbuf],textures:this.texs.length?this.texs:void 0,vertices:this.vbuf,index:this.ibuf??void 0,instances:this.inst,count:this.count,instanceCount:this.n,pickProgram:()=>c,cast:t.cast}]}pickInfo(e){let t=this.drawnOrder[e];if(t===void 0)return null;let n=(this.props.data??[null])[t];return{element:this.id,id:t,props:n&&typeof n==`object`?n:{},d:n,index:e}}animating(){let e=this.props;return/\btime\b/.test(`${e.vertex??``} ${e.fragment??``}`)}detach(){for(let e of[this.prog,this.vbuf,this.ibuf,this.inst,this.ubuf,this.pbuf,...this.texs])e?.destroy();this.prog=null,this.vbuf=this.ibuf=this.inst=this.ubuf=this.pbuf=null,this.texs=[],super.detach()}};function hl(e,t={}){let n=Math.max(1,Math.min(4,t.count??Math.max(1,(typeof navigator<`u`&&navigator.hardwareConcurrency||4)-2))),r=[],i=/* @__PURE__ */ new Map,a=0,o=typeof Worker>`u`,s=e=>{o=!0;for(let[t,n]of i)i.delete(t),n.bad(new K(`E-LANE-001`,`the worker lane stopped: ${e}`,{fix:`check the worker module loads (its imports must not touch the page)`}));for(let e of r)e.w.terminate();r.length=0};if(!o)try{for(let t=0;t<n;t++){let t=typeof e==`function`?e():new Worker(e,{type:`module`}),n={w:t,busy:0};t.onmessage=e=>{let t=i.get(e.data.id);t&&(i.delete(e.data.id),n.busy--,e.data.ok?t.ok(e.data.value):t.bad(new K(`E-LANE-002`,`a worker's call failed: ${e.data.error}`,{fix:`the error is from your worker function`})))},t.onerror=e=>{e.preventDefault?.(),s(e.message||`the worker failed to start`)},r.push(n)}}catch(e){s(String(e?.message??e))}let c=async(e,n)=>{let r=t.fallback?.[e];if(!r)throw new K(`E-LANE-001`,`no worker to run '${e}' and no fallback for it`,{fix:`workers.worker(module, { fallback: { ${e} } })`});return r(...n)},l=(e,n,s)=>{if(o||!r.length)return c(n,s);let l=e===null?r.reduce((e,t)=>t.busy<e.busy?t:e):r[(e%r.length+r.length)%r.length],u=++a;return l.busy++,new Promise((e,t)=>{i.set(u,{ok:e,bad:t,at:performance.now()}),l.w.postMessage({id:u,name:n,args:s})}).catch(e=>{if(o&&t.fallback?.[n])return c(n,s);throw e})};return{call(e,...t){return l(null,e,t)},callIn(e,t,...n){return l(e,t,n)},get size(){return o?0:r.length},get busy(){return i.size},terminate(){s(`terminated`)}}}function gl(e,t={}){let n=globalThis;n.onmessage=async r=>{let{id:i,name:a,args:o}=r.data;try{let r=e[a];if(!r)throw Error(`no function '${a}' (there are ${Object.keys(e).join(`, `)})`);let s=await r(...o);n.postMessage({id:i,ok:!0,value:s},t.transfer===!1?[]:_l(s))}catch(e){n.postMessage({id:i,ok:!1,error:String(e?.stack??e)})}}}function _l(e,t=/* @__PURE__ */ new Set,n=0){if(n>6||typeof e!=`object`||!e)return[...t];if(ArrayBuffer.isView(e))return e.buffer instanceof ArrayBuffer&&t.add(e.buffer),[...t];if(Array.isArray(e)){for(let r of e)_l(r,t,n+1);return[...t]}for(let r of Object.values(e))_l(r,t,n+1);return[...t]}const vl={worker:hl,serve:gl},yl=e=>!!e&&typeof e==`object`&&e.count===!0&&!!e.relations,bl=e=>!!e&&typeof e==`object`&&!!e.members&&!!e.relations,xl=e=>!!e&&typeof e==`object`&&typeof e.formula==`function`,Sl=e=>!!e&&typeof e==`object`&&typeof e.value==`string`;function Cl(e,t,n=[]){let r=/* @__PURE__ */ new Set,i=Object.values(e).flatMap(e=>xl(e)&&e.inputs?[e,...Object.values(e.inputs)]:[e]);for(let e of i)if(typeof e==`string`&&t)r.add(t);else if(bl(e)){let n=e.members.log??t;n&&r.add(n)}else if(Sl(e)){if(typeof e.log==`function`)for(let t of n){let n=e.log(t);n&&r.add(n)}else{let n=e.log??t;n&&r.add(n)}}return[...r]}function wl(e,t,n){let r=e.count[t],i=new Float64Array(r),a=new Float64Array(r);for(let o=0;o<r;o++){let r=e.slot(t,o);i[o]=e.epoch+e.secAt(r)*1e3,a[o]=e.valueAt(n,r)}return{t:i,v:a,step:!!e.values[n].step}}function Tl(e,t,n){let r=e.t.length;if(!r)return 0;for(;n.j+1<r&&e.t[n.j+1]<=t;)n.j++;let i=n.j;if(t<e.t[0])return e.v[0];if(i>=r-1||e.t[i]===t||e.step)return e.v[i];let a=e.t[i],o=e.t[i+1];return o>a?e.v[i]+(e.v[i+1]-e.v[i])*(t-a)/(o-a):e.v[i+1]}function El(e,t){let n=e.t,r=n.length;if(!r)return 0;let i=0,a=r;for(;i<a;){let e=i+a>>1;n[e]<=t?i=e+1:a=e}let o=i-1;return o<0?e.v[0]:o>=r-1||n[o]===t||e.step?e.v[o]:e.v[o]+(e.v[o+1]-e.v[o])*(t-n[o])/(n[o+1]-n[o])}const Dl=(e,t)=>t.length?e===`sum`?t.reduce((e,t)=>e+t,0):e===`mean`?t.reduce((e,t)=>e+t,0)/t.length:e===`min`?Math.min(...t):Math.max(...t):0;function Ol(e,t){let n=e.t,r=n.length;if(!r)return 0;let i=0,a=r;for(;i<a;){let e=i+a>>1;n[e]<t?i=e+1:a=e}if(i>=r)return e.v[r-1];if(e.step)return e.v[i===0?0:i-1];if(n[i]===t||i===0)return e.v[i];let o=n[i-1],s=n[i];return e.v[i-1]+(e.v[i]-e.v[i-1])*(t-o)/(s-o)}const kl=/* @__PURE__ */ new WeakMap;let Al=0;const jl=e=>{let t=kl.get(e);return t===void 0&&kl.set(e,t=++Al),t},Ml=(e,t)=>{if(t<0)return`-`;let n=e.count[t];return`${jl(e)}.${t}.${n}.${e.first[t]}.${e.head[t]}.${n?e.secAt(e.slot(t,n-1)):``}`};function Nl(e,t){if(e===t)return!0;if(!e||!t||typeof e!=`object`||typeof t!=`object`)return!1;let n=Object.keys(e),r=Object.keys(t);if(n.length!==r.length)return!1;for(let r of n){let n=e[r],i=t[r];if(n===i)continue;if(!n||!i||typeof n!=`object`||typeof i!=`object`||Array.isArray(n)||Array.isArray(i))return!1;let a=Object.keys(n),o=Object.keys(i);if(a.length!==o.length)return!1;for(let e of a)if(n[e]!==i[e])return!1}return!0}var Pl=class{chs={};names;o;kept=/* @__PURE__ */ new Map;remade=0;constructor(e,t={}){let n=this.chs;for(let[t,r]of Object.entries(e))if(xl(r)&&r.inputs){let e=Object.keys(r.inputs);for(let i of e)n[`${t}.${i}`]=r.inputs[i];n[t]={formula:(n,i)=>r.formula({...n,...Object.fromEntries(e.map(e=>[e,n[`${t}.${e}`]??0]))},i)}}else n[t]=r;this.names=Object.keys(n),this.o=t;for(let e of this.names){let t=n[e];if(!(typeof t==`number`||typeof t==`function`||typeof t==`string`||Sl(t)||yl(t)||bl(t)||xl(t)))throw new K(`E-DATA-001`,`channel '${e}': ${JSON.stringify(t)} is not a channel`,{fix:`a number, d => number, a log's value ('kw', or { value, log, key }), relations.count, relations.members('mean', 'soc'), or { formula: (v, d) => … }`})}}columnOf(e,t,n){let r=typeof t==`string`?{value:t}:t,i=typeof r.log==`function`?r.log(n)??null:r.log??this.o.log??null;if(!i&&typeof r.log==`function`)return{log:null,k:-1,col:r};if(!i)throw new K(`E-DATA-001`,`channel '${e}' reads '${r.value}' of no log`,{fix:`{ value: '${r.value}', log } (or the reader's log)`});let a=i.values.findIndex(e=>e.name===r.value);if(a<0)throw new K(`E-DATA-001`,`channel '${e}': the log has no value '${r.value}'`,{fix:`its values: ${i.values.map(e=>e.name).join(`, `)||`none`}`});return{log:i,k:a,col:r}}membersLog(e,t){let n=t.members.log??this.o.log;if(!n)throw new K(`E-DATA-001`,`channel '${e}' reads its members' '${t.members.value}' of no log`,{fix:`relations.members('${t.members.agg}', '${t.members.value}', log)`});let r=n.values.findIndex(e=>e.name===t.members.value);if(r<0)throw new K(`E-DATA-001`,`channel '${e}': the log has no value '${t.members.value}'`,{fix:`its values: ${n.values.map(e=>e.name).join(`, `)||`none`}`});return{log:n,k:r}}keysOf(e,t,n){let r=e.key?e.key(t):n;return r==null?[]:Array.isArray(r)?r:[r]}marksOf(e,t){let n=``;for(let r of this.names){let i=this.chs[r];if(yl(i))for(let e of i.relations.membersOf(t))n+=`|${e.of}:${e.from}:${e.until}`;else if(bl(i)){let{log:e}=this.membersLog(r,i);for(let r of i.relations.membersOf(t))n+=`|${r.of}:${r.from}:${r.until}:${Ml(e,e.indexOf(r.of))}`}else if(typeof i==`string`||Sl(i)){let{log:a,col:o}=this.columnOf(r,i,e);if(a)for(let r of this.keysOf(o,e,t))n+=`|${Ml(a,a.indexOf(r))}`}n+=`;`}return n}derive(e){let t=this.names,n=this.o.id??((e,t)=>e.id??t),r=[],i=[],a=/* @__PURE__ */ new Map,o=0,s=0;if(e.forEach((e,t)=>{let c=String(n(e,t)),l=this.marksOf(e,c),u=this.kept.get(c),d=u&&u.marks===l&&Nl(u.d,e)?u:(s++,this.one(e,c,l));r.push(c),i.push(d),a.set(c,d),o+=d.t.length}),this.kept=a,this.remade=s,!s&&this.last&&this.lastIds.length===r.length&&this.lastIds.every((e,t)=>e===r[t]))return this.last;let c=new Float64Array(o),l=new Float64Array(o),u=new Float64Array(o),d=Object.fromEntries(t.map(e=>[e,new Float64Array(o)])),f=[],p=[],m=0,h=1/0;for(let e of i){let n=e.t.length;c.set(e.t,m),l.fill(e.x,m,m+n),u.fill(e.y,m,m+n),t.forEach((t,n)=>d[t].set(e.v[n],m)),f.push(n),p.push({}),m+=n,e.t[0]<h&&(h=e.t[0])}let g=Number.isFinite(h)?h:0;for(let e=0;e<o;e++)Number.isNaN(c[e])&&(c[e]=g);return this.lastIds=r,this.last=Gs.fromColumns({ids:r,count:f,t:c,lon:l,lat:u,values:d,props:p},{values:Object.fromEntries(t.map(e=>[e,`f32`])),place:{type:`f32`},time:{type:`f32`,epoch:g}})}last=null;lastIds=[];one(e,t,n){let r=this.names,i=this.chs,a=this.o,o=r.length,s=new Float64Array(o),c=new Uint8Array(o),l=[],u=[],d=[],f=[],p=[];r.forEach((n,r)=>{let a=i[n];if(typeof a==`number`)s[r]=a,c[r]=1;else if(typeof a==`function`)s[r]=Number(a(e))||0,c[r]=1;else if(xl(a))d.push({k:r,f:a});else if(yl(a)){let e=[];for(let n of a.relations.membersOf(t))e.push([n.from,1],[n.until,-1]);e.sort((e,t)=>e[0]-t[0]||e[1]-t[1]),u.push({k:r,at:e.map(e=>e[0]),by:e.map(e=>e[1]),v:0,i:0});for(let t of e)p.push(t[0])}else if(bl(a)){let{log:e,k:i}=this.membersLog(n,a),o=a.relations.membersOf(t).map(t=>{let n=e.indexOf(t.of);return{from:t.from,until:t.until,tr:n>=0?wl(e,n,i):null}});f.push({k:r,agg:a.members.agg,rows:o});for(let e of o)if(p.push(e.from,e.until),e.tr)for(let t of e.tr.t)t>e.from&&t<e.until&&p.push(t)}else{let{log:i,k:o,col:s}=this.columnOf(n,a,e),c=i?this.keysOf(s,e,t).map(e=>i.indexOf(e)).filter(e=>e>=0).map(e=>wl(i,e,o)):[];l.push({k:r,tr:c,cur:new Int32Array(c.length)});for(let e of c)for(let t of e.t)p.push(t)}}),p.sort((e,t)=>e-t);let m=[];for(let e of p)(!m.length||e!==m[m.length-1])&&m.push(e);m.length||m.push(NaN);let h=Il(a.place?.(e)),g=[],_=r.map(()=>[]),v=new Float64Array(o),y=new Float64Array(o),b={},x=(t,n)=>{if(d.length){for(let e=0;e<o;e++)b[r[e]]=n[e];for(let{k:t,f:i}of d){let a=Number(i.formula(b,e));n[t]=Number.isFinite(a)?a:0,b[r[t]]=n[t]}}g.push(t);for(let e=0;e<o;e++)_[e].push(n[e])},S={j:0},C=[],w=[];for(let e of m){let t=!1;for(let e=0;e<o;e++)c[e]&&(v[e]=s[e],y[e]=s[e]);for(let n of l){let r=0,i=0;for(let t=0;t<n.tr.length;t++)S.j=n.cur[t],r+=Tl(n.tr[t],e,S),n.cur[t]=S.j,i+=Ol(n.tr[t],e);v[n.k]=r,y[n.k]=i,i!==r&&(t=!0)}for(let n of f){C.length=0,w.length=0;for(let t of n.rows)t.tr&&(t.from<=e&&e<t.until&&C.push(El(t.tr,e)),t.from<e&&e<=t.until&&w.push(Ol(t.tr,e)));v[n.k]=Dl(n.agg,C),y[n.k]=Dl(n.agg,w),y[n.k]!==v[n.k]&&(t=!0)}for(let n of u){for(y[n.k]=n.v;n.i<n.at.length&&n.at[n.i]<=e;)n.v+=n.by[n.i],n.i++;v[n.k]=n.v,n.v!==y[n.k]&&(t=!0)}t&&x(e,y),x(e,v)}return{d:e,marks:n,t:Float64Array.from(g),x:h[0],y:h[1],v:_.map(e=>Float64Array.from(e))}}};function Fl(e,t,n={}){return new Pl(t,n).derive(e)}const Il=e=>e&&Number.isFinite(e[0])&&Number.isFinite(e[1])?[e[0],e[1]]:[0,0],Ll={derive:Fl,logsOf:Cl,Deriver:Pl},Rl=e=>typeof e==`function`,zl=e=>Rl(e)||!!e&&typeof e==`object`&&(`regions`in e||`cells`in e);function Bl(e,t){let n=e.by,r=/* @__PURE__ */ new Map,i=[],a=(t,n,r)=>{let i=e.by.kind===`regions`?e.by.ringOff[t]:0,a=e.by.kind===`regions`?e.by.ringOff[t+1]:0,o=e.by.rings,s=!1;for(let e=i,t=a-1;e<a;t=e++){let i=o[2*e],a=o[2*e+1],c=o[2*t],l=o[2*t+1];a>r!=l>r&&n<(c-i)*(r-a)/(l-a)+i&&(s=!s)}return s},o=(e,t,o)=>{if(n.kind===`keys`)return n.key[e];if(n.kind===`cells`){let e=111320*Math.cos(n.lat0*Math.PI/180),a=Math.floor(t*e/n.size),s=Math.floor(o*110574/n.size),c=`${a}_${s}`,l=r.get(c);return l===void 0&&(l=r.size,r.set(c,l),i.push(a,s)),l}for(let e=0;e<n.groups;e++){if(t<n.box[4*e]||t>n.box[4*e+2]||o<n.box[4*e+1]||o>n.box[4*e+3])continue;let r=!1;for(let i=n.groupRings[e];i<n.groupRings[e+1];i++)n.hole[i]?r&&a(i,t,o)&&(r=!1):!r&&a(i,t,o)&&(r=!0);if(r)return e}return-1},s=[];for(let t=0;t<e.n;t++){let n=e.off[t],r=e.off[t+1],i=-2,a=-1/0;for(let c=n;c<r;c++){let l=o(t,e.x[c],e.y[c]),u=r-n>1?e.t[c]:-1/0;l!==i&&(i>=0&&s.push({m:t,g:i,from:a,until:u}),i=l,a=u)}i>=0&&s.push({m:t,g:i,from:a,until:1/0})}let c=n.kind===`cells`?r.size:n.groups,l=Array.from({length:c},()=>[]);for(let e of s)l[e.g].push(e);let u=new Int32Array(c);for(let e=0;e<c;e++)u[e]=new Set(l[e].map(e=>e.m)).size;let d=(t,n,r)=>{let i=e.vOff[t],a=e.vOff[t+1],o=e.vT,s=e.v[n];if(a<=i)return 0;if(r<=o[i])return s[i];if(r>=o[a-1])return s[a-1];let c=i,l=a-1;for(;l-c>1;){let e=c+l>>1;o[e]<=r?c=e:l=e}let u=o[c],d=o[l];return d>u?s[c]+(s[l]-s[c])*(r-u)/(d-u):s[l]},f=[],p=e.ops.map(()=>[]),m=new Int32Array(c+1),h=(e,n,r)=>e===0?n.length:e===5?Number(t?.[r]?.(n))||0:n.length?e===1?n.reduce((e,t)=>e+t,0):e===2?n.reduce((e,t)=>e+t,0)/n.length:e===3?Math.min(...n):e===4?Math.max(...n):0:0,g=1/0;for(let t=0;t<e.t.length;t++)e.t[t]>-1/0&&e.t[t]<g&&(g=e.t[t]);for(let t=0;t<e.vT.length;t++)e.vT[t]<g&&(g=e.vT[t]);Number.isFinite(g)||(g=0);for(let t=0;t<c;t++){m[t]=f.length;let n=l[t],r=[g];for(let e of n)Number.isFinite(e.from)&&r.push(e.from),Number.isFinite(e.until)&&r.push(e.until);if(e.chan.some(e=>e>=0))for(let t of n)for(let n=e.vOff[t.m];n<e.vOff[t.m+1];n++){let i=e.vT[n];i>=t.from&&i<t.until&&r.push(i)}r.sort((e,t)=>e-t);let i=NaN,a=(t,r)=>{f.push(t),e.ops.forEach((i,a)=>{let o=e.chan[a],s=[];for(let e of n)r(e)&&s.push(o>=0?d(e.m,o,t):1);p[a].push(h(i,s,a))})};for(let e of r)e!==i&&(i=e,f.length>m[t]&&n.some(t=>t.from===e||t.until===e)&&a(e,t=>t.from<e&&t.until>=e),a(e,t=>t.from<=e&&t.until>e))}return m[c]=f.length,{groups:c,cells:Int32Array.from(i),members:u,off:m,t:Float64Array.from(f),v:p.map(e=>Float32Array.from(e))}}let Vl=null,Hl=!1;function Ul(e,t){if(t.some(Boolean))return Promise.resolve(Bl(e,t));if(!Hl&&!Vl&&typeof Worker<`u`&&typeof Blob<`u`&&typeof URL<`u`)try{let e=`const groupsJob = ${Bl.toString()};\nself.onmessage = (e) => { const { id, input } = e.data; try { const r = groupsJob(input); self.postMessage({ id, r }, [r.t.buffer, r.off.buffer, r.members.buffer, r.cells.buffer, ...r.v.map((x) => x.buffer)]); } catch (err) { self.postMessage({ id, error: String(err && err.message || err) }); } };`,t=new Worker(URL.createObjectURL(new Blob([e],{type:`text/javascript`}))),n={w:t,seq:0,waiting:/* @__PURE__ */ new Map};t.onmessage=e=>{let t=n.waiting.get(e.data.id);n.waiting.delete(e.data.id),t?.(e.data.r??Error(e.data.error??`the groups worker failed`))},t.onerror=e=>{e.preventDefault?.(),Hl=!0;for(let e of n.waiting.values())e(/* @__PURE__ */ Error(`the groups worker did not start`));n.waiting.clear()},Vl=n}catch{Hl=!0}let n=Vl;if(!n||Hl)return Promise.resolve(Bl(e));let r=++n.seq;return new Promise((t,i)=>{n.waiting.set(r,e=>e instanceof Error?i(e):t(e)),n.w.postMessage({id:r,input:e})}).catch(()=>Bl(e))}var Wl=class extends Te{level;log=new Gs({keep:1/0});make;constructor(e,t,n){super({id:e}),this.level=t,this.make=n}async load(e){let t=await this.make(e);this.log=t.log,this.replace(t.features)}async again(e){let t=await this.track(this.make(e));this.log=t.log,this.replace(t.features)}};const Gl={count:0,sum:1,mean:2,min:3,max:4};var Kl=class extends D{kind=`custom`;sources=[];marks=[];members=null;memberWhen;wrapped=!1;ctx=null;constructor(e){super(e);let t=e.by,n=e.show?.levels??[];if(!t)throw new K(`E-DATA-001`,`groups '${this.id}' needs a grouping: by`,{where:this.id,fix:`by: { regions: suburbs } · { cells: 500 } · d => d.props.district`});let r=!zl(t);if(r)for(let e of Object.keys(t)){if(!n.includes(e))throw new K(`E-DATA-001`,`groups '${this.id}': a grouping for '${e}', not a level it shows`,{where:this.id,fix:`show: { levels: [${[...n,e].map(e=>`'${e}'`).join(`, `)}] }`});if(!zl(t[e]))throw new K(`E-DATA-001`,`groups '${this.id}': by.${e} is not a grouping`,{where:this.id,fix:`{ regions } · { cells } · a function`})}let i=r?Object.entries(t):[[null,t]];for(let[t,r]of i){let i=new Wl(`${this.id}${t?`-${t}`:``}`,t,e=>this.build(r,t,e));this.sources.push(i);let a=e.mark?e.mark(i,t):ql(i,e.values??{n:{count:!0}});n.length&&a.props.when===void 0&&(a.props.when={level:t||n}),a.owner=this,this.marks.push(a)}}companions(){return this.marks}async attach(e){await super.attach(e),this.ctx=e;let t=this.props.show?.levels,n=this.props.of,r=e.world;if(this.members=typeof n==`string`&&r.element?r.element(n):n instanceof D?n:null,t?.length&&this.members&&!this.wrapped){let e=this.members,n=e.props.when;this.memberWhen=n,this.wrapped=!0,e.setProps({when:r=>{let i=r.observation;return i&&i.level!==null&&An(t,i.levels,i.index,`groups '${this.id}'`)?0:j(n,r,`element '${e.id}'`)}})}for(let t of this.sources)t.start({loader:e.loader,error:t=>e.world.error(t)})}detach(){this.members&&this.wrapped&&(this.members.setProps({when:this.memberWhen}),this.wrapped=!1),this.ctx=null,super.detach()}async build(e,t,n){let r=this.props,i=this.ctx?.world,a=r.of;if(typeof a==`string`){if(!i?.element)throw new K(`E-DATA-001`,`groups '${this.id}': no element '${a}'`,{where:this.id});a=i.element(a)}let o=[],s=null;if(a instanceof bc)s=a,await s.log.start({loader:n.loader,error:n.error}),o=s.log.ids.map((e,t)=>({id:e,props:s.log.props[t]??{},index:t}));else{let e=a;if(a instanceof D){let t=a.props;e=t.data??t.of}if(e instanceof Te&&(await e.start(n),e=e.features()),!Array.isArray(e))throw new K(`E-DATA-001`,`groups '${this.id}': its members have no features`,{where:this.id,fix:`of: a set, a layer with data, a source or an array`});o=e.map(e=>e.props||!e.properties?e:qe([e])[0])}let c=o.length,l=new Int32Array(c+1),u=[],d=[],f=[],p=a instanceof D?a.props.at:void 0;o.forEach((e,t)=>{if(l[t]=d.length,s){let e=s.log;for(let n=0;n<e.count[t];n++){let r=e.slot(t,n),i=e.relAt(r),a=e.lonLat(i[0],i[1]);u.push(e.epoch+e.secAt(r)*1e3),d.push(a[0]),f.push(a[1])}}else{let t=p?p(e):Ve(e.geometry??null);t&&(u.push(-1/0),d.push(t[0]),f.push(t[1]))}}),l[c]=d.length;let m,h=[],g=[];if(Rl(e)){let t=/* @__PURE__ */ new Map,n=new Int32Array(c);o.forEach((r,i)=>{let a=e(r);if(a==null){n[i]=-1;return}let o=String(a),s=t.get(o);s===void 0&&(s=t.size,t.set(o,s),g.push(a)),n[i]=s}),m={kind:`keys`,key:n,groups:t.size}}else if(`cells`in e){if(!(e.cells>0))throw new K(`E-DATA-001`,`groups '${this.id}': cells of ${e.cells} m`,{where:this.id,fix:`by: { cells: 500 } (metres)`});let t=0;for(let e=0;e<f.length;e++)t+=f[e];t=f.length?t/f.length:0,m={kind:`cells`,size:e.cells,lat0:t}}else{let t=e.regions;t instanceof Te&&await t.start(n),h=(t instanceof Te?t.features():t).map(e=>e.props||!e.properties?e:qe([e])[0]).filter(e=>Xe(e.geometry).length);let r=[],i=[0],a=[0],o=[],s=[];h.forEach(e=>{for(let t of Xe(e.geometry))t.forEach((e,t)=>{for(let t of e)r.push(t[0],t[1]);i.push(r.length/2),o.push(+(t>0))});a.push(o.length);let t=O(e.geometry)??[0,0,0,0];s.push(...t)}),m={kind:`regions`,groups:h.length,rings:Float64Array.from(r),ringOff:Int32Array.from(i),groupRings:Int32Array.from(a),hole:Uint8Array.from(o),box:Float64Array.from(s)}}let _=Object.entries(r.values??{n:{count:!0}}),v={},y=[],b=[];for(let[e,t]of _){let n=`count`in t?`count`:`sum`in t?`sum`:`mean`in t?`mean`:`min`in t?`min`:`max`in t?`max`:`fn`in t?`fn`:null;if(!n)throw new K(`E-DATA-001`,`groups '${this.id}': value '${e}' is not an aggregate`,{where:this.id,fix:`{ count: true } · { sum: ch } · { mean: ch } · { min: ch } · { max: ch } · { of: ch, fn }`});if(n===`count`){y.push(0),b.push(-1);continue}let r=n===`fn`?t.of:t[n];b.push(Object.keys(v).length),v[`c${Object.keys(v).length}`]=r,y.push(n===`fn`?5:Gl[n])}let x=r.log??s?.log??null;for(let e of Cl(v,x,o))await e.start({loader:n.loader,error:n.error});let S=Object.keys(v).length?Fl(o,v,{log:x,id:(e,t)=>e.id??t}):null,C=new Int32Array(c+1),w=[],T=Object.keys(v).map(()=>[]);for(let e=0;e<c;e++)if(C[e]=w.length,S)for(let t=0;t<S.count[e];t++){let n=S.first[e]+t;w.push(S.epoch+S.secAt(n)*1e3),T.forEach((e,t)=>e.push(S.cols[t][n]))}C[c]=w.length;let E=await Ul({n:c,off:l,t:Float64Array.from(u),x:Float64Array.from(d),y:Float64Array.from(f),by:m,ops:y,chan:b,vOff:C,vT:Float64Array.from(w),v:T.map(e=>Float32Array.from(e))},_.map(([,e])=>`fn`in e?e.fn:null)),k=E.groups,A=[],j=[],M=[],N=[],P=[],F=m.kind===`cells`?m.lat0:0,I=m.kind===`cells`?m.size:0;for(let n=0;n<k;n++){let r,i,a;if(m.kind===`regions`){let t=h[n];r=t.geometry,a=String(t.id),i=String(e.name?.(t)??t.props.name??t.id)}else if(m.kind===`cells`){let e=E.cells[2*n],t=E.cells[2*n+1],o=111320*Math.cos(F*G),s=e*I/o,c=t*I/110574,l=(e+1)*I/o,u=(t+1)*I/110574;r={type:`Polygon`,coordinates:[[[s,c],[l,c],[l,u],[s,u],[s,c]]]},a=`${e}_${t}`,i=a}else{let e=0,t=0,s=0;o.forEach((r,i)=>{if(m.key[i]===n)for(let n=l[i];n<l[i+1];n++)e+=d[n],t+=f[n],s++}),r={type:`Point`,coordinates:[s?e/s:0,s?t/s:0]},a=String(g[n]),i=a}let s=Ve(r)??[0,0],c=E.off[n],u={id:a,name:i,level:t,members:E.members[n]};if(_.forEach(([e],t)=>{u[e]=E.v[t][c]??0}),!E.members[n])continue;A.push(a),j.push({id:a,props:u,geometry:r});let p=E.off[n+1]-c;P.push(p);for(let e=0;e<p;e++)M.push(s[0]),N.push(s[1])}let L=[];for(let e=0;e<k;e++)if(E.members[e])for(let t=E.off[e];t<E.off[e+1];t++)L.push(t);return{features:j,log:Gs.fromColumns({ids:A,count:P,t:L.map(e=>E.t[e]),lon:M,lat:N,values:Object.fromEntries(_.map(([e],t)=>[e,L.map(e=>E.v[t][e])])),props:j.map(e=>e.props)},{values:Object.fromEntries(_.map(([e])=>[e,`f32`])),place:{type:`f32`},time:{type:`f32`}})}}frame(e){return[]}status(){let e=super.status();for(let t of this.sources){let n=t.status();e.pending.push(...n.pending),e.failed.push(...n.failed)}return e.ready=!e.pending.length,e}readouts(){return{groups:()=>this.sources.map(e=>e.size).join(`/`)}}};function ql(e,t){let n=Object.keys(t)[0];return lt({id:`${e.id}-labels`,data:e,text:e=>n?`${e.props.name}\n${Math.round(Number(e.props[n])*10)/10}`:String(e.props.name),priority:e=>Number(n?e.props[n]:e.props.members)||0,size:13,color:`#1f2937`,halo:{width:3,color:`#ffffffe6`}})}function Jl(e){return new Kl(e)}const Yl={SCALAR:1,VEC2:2,VEC3:3,VEC4:4,MAT4:16},Xl=(e,t)=>{try{return new URL(t,new URL(e,typeof location<`u`?location.href:`http://x/`)).href}catch{return t}};function Zl(e,t,n){let r=e.accessors[n],i=Yl[r.type]??1,a=r.bufferView===void 0?null:e.bufferViews[r.bufferView],o=r.componentType===5125||!r.normalized&&(r.componentType===5121||r.componentType===5123)?new Uint32Array(r.count*i):new Float32Array(r.count*i);if(!a)return{data:o,size:i,count:r.count};let s=t[a.buffer],c=new DataView(s),l={5120:1,5121:1,5122:2,5123:2,5125:4,5126:4}[r.componentType]??4,u=a.byteStride??i*l,d=(a.byteOffset??0)+(r.byteOffset??0);for(let e=0;e<r.count;e++)for(let t=0;t<i;t++){let n=d+e*u+t*l,a;switch(r.componentType){case 5126:a=c.getFloat32(n,!0);break;case 5125:a=c.getUint32(n,!0);break;case 5123:a=c.getUint16(n,!0),r.normalized&&(a/=65535);break;case 5122:a=c.getInt16(n,!0),r.normalized&&(a=Math.max(-1,a/32767));break;case 5121:a=c.getUint8(n),r.normalized&&(a/=255);break;default:a=c.getInt8(n),r.normalized&&(a=Math.max(-1,a/127))}o[e*i+t]=a}return{data:o,size:i,count:r.count}}function Ql(e){if(e.matrix)return e.matrix;let t=e.rotation??[0,0,0,1],n=e.scale??[1,1,1],r=e.translation??[0,0,0],i=t[0],a=t[1],o=t[2],s=t[3],c=n[0],l=n[1],u=n[2];return[(1-2*(a*a+o*o))*c,2*(i*a+o*s)*c,2*(i*o-a*s)*c,0,2*(i*a-o*s)*l,(1-2*(i*i+o*o))*l,2*(a*o+i*s)*l,0,2*(i*o+a*s)*u,2*(a*o-i*s)*u,(1-2*(i*i+a*a))*u,0,r[0],r[1],r[2],1]}const $l=(e,t)=>{let n=Array(16).fill(0);for(let r=0;r<4;r++)for(let i=0;i<4;i++)for(let a=0;a<4;a++)n[r*4+i]+=e[a*4+i]*t[r*4+a];return n};async function eu(e,t,n,r,i){let a=e.images?.[r];if(!a||typeof createImageBitmap>`u`||typeof OffscreenCanvas>`u`)return null;let o;if(a.bufferView!==void 0){let n=e.bufferViews[a.bufferView];o=t[n.buffer].slice(n.byteOffset??0,(n.byteOffset??0)+n.byteLength)}else if(a.uri)o=await i(Xl(n,a.uri));else return null;let s=await createImageBitmap(new Blob([o],{type:a.mimeType??`image/png`}),{colorSpaceConversion:`none`,premultiplyAlpha:`none`}),c=new OffscreenCanvas(s.width,s.height).getContext(`2d`);return c.drawImage(s,0,0),{w:s.width,h:s.height,px:c.getImageData(0,0,s.width,s.height).data}}async function tu(e,t={}){let n=t.fetch??(async e=>{let t=await fetch(e);if(!t.ok)throw new K(`E-DATA-002`,`nothing at ${e} (${t.status})`,{fix:`check the URL (a model kept outside git, served with the data)`});return t.arrayBuffer()}),r=await n(e),i=new Uint8Array(r,0,4),a,o=[];if(i[0]===103&&i[1]===108&&i[2]===84&&i[3]===70){let e=new DataView(r),t=e.getUint32(12,!0);a=JSON.parse(new TextDecoder().decode(new Uint8Array(r,20,t)));let n=20+t;if(n+8<=r.byteLength){let t=e.getUint32(n,!0);o.push(r.slice(n+8,n+8+t))}}else a=JSON.parse(new TextDecoder().decode(new Uint8Array(r)));if(!a.meshes?.length)throw new K(`E-DATA-002`,`${e} has no meshes`,{fix:`a glTF 2.0 model with triangles`});for(let t=0;t<(a.buffers?.length??0);t++){let r=a.buffers[t].uri;r&&o[t]===void 0&&(o[t]=await n(r.startsWith(`data:`)?r:Xl(e,r)))}let s=await Promise.all((a.materials??[]).map(async t=>{let r=t.pbrMetallicRoughness??{},i=r.baseColorFactor??[1,1,1,1],s=r.baseColorTexture?.index,c=s===void 0?void 0:a.textures?.[s]?.source;return{f:i,tex:c===void 0?null:await eu(a,o,e,c,n).catch(()=>null)}})),c=[],l=[],u=[],d=(e,t)=>{let n=a.nodes[e],r=$l(t,Ql(n));if(n.mesh!==void 0)for(let e of a.meshes[n.mesh].primitives){if((e.mode??4)!==4||e.attributes.POSITION===void 0)continue;let t=Zl(a,o,e.attributes.POSITION),n=e.attributes.NORMAL===void 0?null:Zl(a,o,e.attributes.NORMAL),i=e.attributes.TEXCOORD_0===void 0?null:Zl(a,o,e.attributes.TEXCOORD_0),d=e.indices===void 0?Uint32Array.from({length:t.count},(e,t)=>t):Zl(a,o,e.indices).data,f=e.material===void 0?null:s[e.material],p=f?.f??[1,1,1,1],m=f?.tex??null;for(let e=0;e+2<d.length;e+=3)for(let a=0;a<3;a++){let o=d[e+a],s=t.data[3*o],f=t.data[3*o+1],h=t.data[3*o+2];c.push(r[0]*s+r[4]*f+r[8]*h+r[12],r[1]*s+r[5]*f+r[9]*h+r[13],r[2]*s+r[6]*f+r[10]*h+r[14]);let g=n?n.data[3*o]:0,_=n?n.data[3*o+1]:1,v=n?n.data[3*o+2]:0,y=r[0]*g+r[4]*_+r[8]*v,b=r[1]*g+r[5]*_+r[9]*v,x=r[2]*g+r[6]*_+r[10]*v,S=Math.hypot(y,b,x)||1;l.push(y/S,b/S,x/S);let C=p[0],w=p[1],T=p[2];if(m&&i){let e=i.data[2*o],t=i.data[2*o+1],n=Math.min(m.w-1,Math.max(0,Math.floor((e%1+1)%1*m.w))),r=(Math.min(m.h-1,Math.max(0,Math.floor((t%1+1)%1*m.h)))*m.w+n)*4;C*=m.px[r]/255,w*=m.px[r+1]/255,T*=m.px[r+2]/255}u.push(C,w,T)}}for(let e of n.children??[])d(e,r)},f=a.scenes?.[a.scene??0]?.nodes??a.nodes?.map((e,t)=>t)??[];for(let e of f)d(e,[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);let p=c.length/3;if(!p)throw new K(`E-DATA-002`,`${e} has no triangles`,{fix:`a glTF 2.0 model with triangle meshes`});let m=(t.turn??0)*Math.PI/180,h=Math.cos(m),g=Math.sin(m);for(let e=0;e<p;e++)for(let t of[c,l]){let n=t[3*e],r=t[3*e+2];t[3*e]=n*h+r*g,t[3*e+2]=-n*g+r*h}let _=[1/0,1/0,1/0],v=[-1/0,-1/0,-1/0];for(let e=0;e<p;e++)for(let t=0;t<3;t++){let n=c[3*e+t];n<_[t]&&(_[t]=n),n>v[t]&&(v[t]=n)}let y=t.scale??(t.length?t.length/Math.max(1e-9,Math.max(v[0]-_[0],v[2]-_[2])):1),b=(_[0]+v[0])/2,x=(_[2]+v[2])/2,S=e=>Math.round(u[3*e]*31)<<10|Math.round(u[3*e+1]*31)<<5|Math.round(u[3*e+2]*31),C=t.tolerance??.06,w=(e,t,n)=>{let r=Math.max(e,t,n,1e-6);return[e/r,t/r,n/r,(r-Math.min(e,t,n))/r,r]},T=(e,t)=>e[3]>.2==t[3]>.2&&(e[3]>.2?Math.abs(e[0]-t[0])+Math.abs(e[1]-t[1])+Math.abs(e[2]-t[2])<=C*4:Math.abs(e[4]-t[4])<=C*2),E=null;if(t.paint!==!1){if(t.paint&&t.paint!==`auto`){let e=L(t.paint);E=w(e[0],e[1],e[2])}else{let e=/* @__PURE__ */ new Map,t=0;for(let n=0;n<p;n+=3){let r=c[3*n],i=c[3*n+1],a=c[3*n+2],o=c[3*n+3]-r,s=c[3*n+4]-i,l=c[3*n+5]-a,d=c[3*n+6]-r,f=c[3*n+7]-i,p=c[3*n+8]-a,m=Math.hypot(s*p-l*f,l*d-o*p,o*f-s*d)/2,h=S(n),g=e.get(h);g?g.a+=m:e.set(h,{a:m,c:w(u[3*n],u[3*n+1],u[3*n+2])}),t+=m}let n=[...e.values()],r=n.map(e=>({x:e,a:n.reduce((t,n)=>t+(T(e.c,n.c)?n.a:0),0)})),i=r.filter(e=>e.x.c[3]>.2).sort((e,t)=>t.a-e.a)[0],a=[...r].sort((e,t)=>t.a-e.a)[0],o=i&&i.a>t*.12?i:a;o&&(E=n.filter(e=>T(o.x.c,e.c)).reduce((e,t)=>t.c[4]>e[4]?t.c:e,o.x.c))}}let D=new Float32Array(p*6);for(let e=0;e<p;e++){let t=e*6,n=u[3*e],r=u[3*e+1],i=u[3*e+2],a=w(n,r,i);D[t]=(c[3*e]-b)*y,D[t+1]=(c[3*e+1]-_[1])*y,D[t+2]=(c[3*e+2]-x)*y,D[t+3]=F(l[3*e],l[3*e+1],l[3*e+2]);let o=!!E&&T(a,E),s=o?Math.min(1,a[4]/Math.max(1e-6,E[4])):1;D[t+4]=W(o?[s,s,s]:[n,r,i]),D[t+5]=o?0:E?256:0}let O=[[(_[0]-b)*y,0,(_[2]-x)*y],[(v[0]-b)*y,(v[1]-_[1])*y,(v[2]-x)*y]],k={levels:[D,D,D],parts:[],anchors:[],triangles:[p/3,p/3,p/3],bounds:O};return t.kind===void 0?(t.params&&(k.params={...t.params}),t.anchors&&(k.anchors=Je(typeof t.anchors==`function`?t.anchors(k):t.anchors)),k):Ge(k,t.kind,{anchors:t.anchors,params:t.params})}const nu={import:tu},ru=(e=`
fn skinDecal(uv: vec2f, p: vec3f, base: vec3f, seed: f32) -> vec3f {
  return base;
}
`)=>`${Nt}
@group(1) @binding(4) var bones: texture_2d<f32>;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) col: vec3f,
  @location(1) n: vec3f,
  @location(2) world: vec3f,
  @location(3) @interpolate(flat) id: f32,
  @location(4) uv: vec2f,
  @location(5) @interpolate(flat) dp: vec4f,
}
${e}
// a texel of the skin transforms: a row, a column
fn texel(row: f32, x: f32) -> vec4f {
  return textureLoad(bones, vec2i(i32(x + 0.5), i32(row + 0.5)), 0);
}
// a rest point moved by a unit dual quaternion (r: rotation, d: its dual part)
fn dqPoint(r: vec4f, d: vec4f, p: vec3f) -> vec3f {
  return p + 2.0 * cross(r.xyz, cross(r.xyz, p) + r.w * p) + 2.0 * (r.w * d.xyz - d.w * r.xyz + cross(r.xyz, d.xyz));
}
fn dqVector(r: vec4f, v: vec3f) -> vec3f {
  return v + 2.0 * cross(r.xyz, cross(r.xyz, v) + r.w * v);
}
// two morph offsets' numbers out of one float (12 bits each, ±0.2 m in 0.1 mm)
fn unpack12(v: f32) -> vec2f {
  let hi = floor(v / 4096.0);
  return (vec2f(hi, v - hi * 4096.0) - vec2f(2048.0, 2048.0)) / 10000.0;
}
struct I { @location(0) i0: vec4f, @location(1) i1: vec4f, @location(2) i2: vec4f, @location(3) i3: vec4f, @location(4) i4: vec4f, @location(5) a0: vec4f, @location(6) a1: vec4f, @location(7) a2: vec4f, @location(8) a3: vec4f }
@vertex fn vs(i: I) -> O {
  t = tb;
  let A = i.i1.x;
  let B = i.i1.y;
  let tw = i.i1.z;
  var js = array<f32, 4>(i.a1.x - floor(i.a1.x / 256.0) * 256.0, floor(i.a1.x / 256.0), i.a1.y - floor(i.a1.y / 256.0) * 256.0, floor(i.a1.y / 256.0));
  var ws = array<f32, 4>(i.a2.x, i.a2.y, i.a2.z, i.a2.w);
  var r = vec4f(0.0);
  var d = vec4f(0.0);
  var r0 = vec4f(0.0, 0.0, 0.0, 1.0);
  for (var k = 0; k < 4; k++) {
    if (k > 0 && ws[k] <= 0.0) {
      continue;
    }
    // the bone between the two rows (the shorter way round)
    let bx = js[k] * 2.0;
    let ra = texel(A, bx);
    var rb = texel(B, bx);
    let da = texel(A, bx + 1.0);
    var db = texel(B, bx + 1.0);
    if (dot(ra, rb) < 0.0) {
      rb = -rb;
      db = -db;
    }
    let qr = mix(ra, rb, tw);
    let qd = mix(da, db, tw);
    if (k == 0) {
      r0 = qr;
    }
    // in the first bone's hemisphere
    let w = select(ws[k], -ws[k], dot(qr, r0) < 0.0);
    r += qr * w;
    d += qd * w;
  }
  let len = max(length(r), 1e-6);
  r /= len;
  d /= len;
  var p = i.a0.xyz;
  if (i.a3.w > -0.5) {
    let nb = f32(textureDimensions(bones, 0).x) - 1.0;
    let mw = mix(texel(A, nb), texel(B, nb), tw);
    let pw = select(mw.xy, mw.zw, i.a3.w > 0.5);
    let ax = unpack12(i.a3.x);
    let ay = unpack12(i.a3.y);
    let az = unpack12(i.a3.z);
    p += vec3f(ax.x, ax.y, ay.x) * pw.x + vec3f(ay.y, az.x, az.y) * pw.y;
  }
  let q = dqPoint(r, d, p) * i.i1.w;
  let nq = dqVector(r, unpackNormal(i.a0.w));
  let c = cos(i.i0.w);
  let s = sin(i.i0.w);
  var position = placeOf(i.i0.x + q.x * c - q.z * s, i.i0.z + q.y, i.i0.y + q.x * s + q.z * c);
  var pal = array<f32, 8>(i.i2.x, i.i2.y, i.i2.z, i.i2.w, i.i3.x, i.i3.y, i.i3.z, i.i3.w);
  var o: O;
  o.col = unpackRGB(pal[clamp(i32(i.a1.z + 0.5), 0, 7)]);
  o.n = vec3f(nq.x * c - nq.z * s, nq.y, nq.x * s + nq.z * c);
  o.id = i.i4.x;
  let hu = floor(i.a1.w / 4096.0);
  o.uv = select(vec2f(-1.0, -1.0), vec2f(hu, i.a1.w - hu * 4096.0) / 4095.0, i.a1.w >= 0.0);
  o.dp = vec4f(i.i4.yzw, i.i4.x);
  o.pos = clipOut(frame.viewProj * vec4f(position, 1.0));
  o.world = position;
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  t = tb;
  if (t.v.w < 1.0 && dither(i.pos.xy) > t.v.w) {
    discard;
  }
  if (frame.pick.y > 0.5) {
    return packDepth(i.pos.z);
  }
  if (frame.pick.x > 0.5) {
    return pickOut(t.v.y, t.w.w * 65536.0 + i.id);
  }
  var base = i.col;
  if (i.uv.x >= 0.0) {
    base = skinDecal(i.uv, i.dp.xyz, base, i.dp.w);
  }
  return vec4f(shadeSolid(base, normalize(i.n), i.world, i.id, 0.0), 1.0);
}
`;ru();const iu=(e=`
vec3 skinDecal(vec2 uv, vec3 p, vec3 base, float seed) {
  return base;
}
`)=>({vertex:`${Wt}
uniform highp sampler2D bones;
out vec2 v_uv;
flat out vec4 v_dp;
vec4 texel(float row, float x) {
  return texelFetch(bones, ivec2(int(x + 0.5), int(row + 0.5)), 0);
}
vec3 dqPoint(vec4 r, vec4 d, vec3 p) {
  return p + 2.0 * cross(r.xyz, cross(r.xyz, p) + r.w * p) + 2.0 * (r.w * d.xyz - d.w * r.xyz + cross(r.xyz, d.xyz));
}
vec3 dqVector(vec4 r, vec3 v) {
  return v + 2.0 * cross(r.xyz, cross(r.xyz, v) + r.w * v);
}
vec2 unpack12(float v) {
  float hi = floor(v / 4096.0);
  return (vec2(hi, v - hi * 4096.0) - vec2(2048.0, 2048.0)) / 10000.0;
}
layout(location = 0) in vec4 i0;
layout(location = 1) in vec4 i1;
layout(location = 2) in vec4 i2;
layout(location = 3) in vec4 i3;
layout(location = 4) in vec4 i4;
layout(location = 5) in vec4 a0;
layout(location = 6) in vec4 a1;
layout(location = 7) in vec4 a2;
layout(location = 8) in vec4 a3;
void main() {
  t = Tile(tb.o, tb.v, tb.w, tb.l);
  float A = i1.x;
  float B = i1.y;
  float tw = i1.z;
  float js[4] = float[4](a1.x - floor(a1.x / 256.0) * 256.0, floor(a1.x / 256.0), a1.y - floor(a1.y / 256.0) * 256.0, floor(a1.y / 256.0));
  float ws[4] = float[4](a2.x, a2.y, a2.z, a2.w);
  vec4 r = vec4(0.0);
  vec4 d = vec4(0.0);
  vec4 r0 = vec4(0.0, 0.0, 0.0, 1.0);
  for (int k = 0; k < 4; k++) {
    if (k > 0 && ws[k] <= 0.0) {
      continue;
    }
    float bx = js[k] * 2.0;
    vec4 ra = texel(A, bx);
    vec4 rb = texel(B, bx);
    vec4 da = texel(A, bx + 1.0);
    vec4 db = texel(B, bx + 1.0);
    if (dot(ra, rb) < 0.0) {
      rb = -rb;
      db = -db;
    }
    vec4 qr = mix(ra, rb, tw);
    vec4 qd = mix(da, db, tw);
    if (k == 0) {
      r0 = qr;
    }
    float w = dot(qr, r0) < 0.0 ? -ws[k] : ws[k];
    r += qr * w;
    d += qd * w;
  }
  float len = max(length(r), 1e-6);
  r /= len;
  d /= len;
  vec3 p = a0.xyz;
  if (a3.w > -0.5) {
    float nb = float(textureSize(bones, 0).x) - 1.0;
    vec4 mw = mix(texel(A, nb), texel(B, nb), tw);
    vec2 pw = a3.w > 0.5 ? mw.zw : mw.xy;
    vec2 ax = unpack12(a3.x);
    vec2 ay = unpack12(a3.y);
    vec2 az = unpack12(a3.z);
    p += vec3(ax.x, ax.y, ay.x) * pw.x + vec3(ay.y, az.x, az.y) * pw.y;
  }
  vec3 q = dqPoint(r, d, p) * i1.w;
  vec3 nq = dqVector(r, unpackNormal(a0.w));
  float c = cos(i0.w);
  float s = sin(i0.w);
  vec3 position = placeOf(i0.x + q.x * c - q.z * s, i0.z + q.y, i0.y + q.x * s + q.z * c);
  float pal[8] = float[8](i2.x, i2.y, i2.z, i2.w, i3.x, i3.y, i3.z, i3.w);
  v_col = unpackRGB(pal[clamp(int(a1.z + 0.5), 0, 7)]);
  v_n = vec3(nq.x * c - nq.z * s, nq.y, nq.x * s + nq.z * c);
  v_id = i4.x;
  v_leaf = vec3(0.0);
  float hu = floor(a1.w / 4096.0);
  v_uv = a1.w >= 0.0 ? vec2(hu, a1.w - hu * 4096.0) / 4095.0 : vec2(-1.0, -1.0);
  v_dp = vec4(i4.yzw, i4.x);
  gl_Position = clipOut(frame.viewProj * vec4(position, 1.0));
  v_world = position;
}
`,fragment:`${ut}
${e}
in vec3 v_col;
in vec3 v_n;
in vec3 v_world;
flat in float v_id;
in vec2 v_uv;
flat in vec4 v_dp;
out vec4 outColor;
void main() {
  t = Tile(tb.o, tb.v, tb.w, tb.l);
  if (t.v.w < 1.0 && dither(gl_FragCoord.xy) > t.v.w) {
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
  vec3 base = v_col;
  if (v_uv.x >= 0.0) {
    base = skinDecal(v_uv, v_dp.xyz, base, v_dp.w);
  }
  outColor = vec4(shadeSolid(base, normalize(v_n), v_world, v_id, 0.0), 1.0);
}
`});iu();const au=8;var ou=class{names;parent;head;byName=/* @__PURE__ */ new Map;constructor(e){if(!e?.length)throw new K(`E-DATA-001`,`a skeleton needs bones`,{fix:`skeleton([{ name: 'hips', head: [0, 1, 0] }, …])`});e.forEach((e,t)=>this.byName.set(e.name,t)),this.names=e.map(e=>e.name),this.parent=Int16Array.from(e,(e,t)=>{if(!e.parent)return-1;let n=this.byName.get(e.parent);if(n===void 0||n>=t)throw new K(`E-DATA-001`,`bone '${e.name}': its parent '${e.parent}' must come before it`,{fix:`list parents before their children`});return n}),this.head=Float32Array.from(e.flatMap(e=>[e.head[0],e.head[1],e.head[2]]))}get count(){return this.names.length}index(e){return this.byName.get(e)??-1}rest(){let e=new Float32Array(this.count*4);for(let t=0;t<this.count;t++)e[t*4+3]=1;return e}solve(e,t=[0,0,0],n){let r=this.count,i=n?.q??new Float32Array(r*4),a=n?.p??new Float32Array(r*3),o=this.head,s=[0,0,0];for(let n=0;n<r;n++){let r=this.parent[n];r<0?(i.set(e.subarray(n*4,n*4+4),n*4),a[n*3]=o[n*3]+t[0],a[n*3+1]=o[n*3+1]+t[1],a[n*3+2]=o[n*3+2]+t[2]):(uu(s,i,r*4,o[n*3]-o[r*3],o[n*3+1]-o[r*3+1],o[n*3+2]-o[r*3+2]),a[n*3]=a[r*3]+s[0],a[n*3+1]=a[r*3+1]+s[1],a[n*3+2]=a[r*3+2]+s[2],cu(i,n*4,i,r*4,e,n*4))}return{q:i,p:a}}dual(e,t=[0,0,0],n,r=0){let i=this.count,a=n??new Float32Array(i*8),{q:o,p:s}=this.solve(e,t,this.scratch()),c=[0,0,0],l=this.head;for(let e=0;e<i;e++){uu(c,o,e*4,l[e*3],l[e*3+1],l[e*3+2]);let t=s[e*3]-c[0],n=s[e*3+1]-c[1],i=s[e*3+2]-c[2],u=o[e*4],d=o[e*4+1],f=o[e*4+2],p=o[e*4+3],m=r+e*8;a[m]=u,a[m+1]=d,a[m+2]=f,a[m+3]=p,a[m+4]=.5*(t*p+n*f-i*d),a[m+5]=.5*(n*p+i*u-t*f),a[m+6]=.5*(i*p+t*d-n*u),a[m+7]=-.5*(t*u+n*d+i*f)}return a}sc=null;scratch(){return this.sc??={q:new Float32Array(this.count*4),p:new Float32Array(this.count*3)}}};const su=e=>new ou(e);function cu(e,t,n,r,i,a){let o=n[r],s=n[r+1],c=n[r+2],l=n[r+3],u=i[a],d=i[a+1],f=i[a+2],p=i[a+3];e[t]=l*u+o*p+s*f-c*d,e[t+1]=l*d-o*f+s*p+c*u,e[t+2]=l*f+o*d-s*u+c*p,e[t+3]=l*p-o*u-s*d-c*f}function lu(e,t){let n=Math.hypot(e[0],e[1],e[2])||1,r=Math.sin(t/2)/n;return[e[0]*r,e[1]*r,e[2]*r,Math.cos(t/2)]}function uu(e,t,n,r,i,a){let o=t[n],s=t[n+1],c=t[n+2],l=t[n+3],u=s*a-c*i+l*r,d=c*r-o*a+l*i,f=o*i-s*r+l*a;e[0]=r+2*(s*f-c*d),e[1]=i+2*(c*u-o*f),e[2]=a+2*(o*d-s*u)}function du(e,t,n,r){if(t<0||!r)return;let i=lu(n,r);cu(e,t*4,i,0,e,t*4)}function fu(e,t,n,r,i,a,o,s,c=!1){let l=0,u=0,d=0,f=0,p=0,m=0,h=0,g=0,_=0,v=0,y=0,b=1,x=!0;if(c){let c=0,l=0,u=0,d=[0,0,0];for(let e=0;e<4;e++){let f=a[o*4+e];if(!f)continue;let p=i[o*4+e]*8;uu(d,s,p,t,n,r);let m=pu(s,p);c+=f*(d[0]+m[0]),l+=f*(d[1]+m[1]),u+=f*(d[2]+m[2])}return e[0]=c,e[1]=l,e[2]=u,e}for(let e=0;e<4;e++){let t=a[o*4+e];if(!t)continue;let n=i[o*4+e]*8,r=s[n],c=s[n+1],S=s[n+2],C=s[n+3];x&&=(_=r,v=c,y=S,b=C,!1),r*_+c*v+S*y+C*b<0&&(t=-t),l+=t*r,u+=t*c,d+=t*S,f+=t*C,p+=t*s[n+4],m+=t*s[n+5],h+=t*s[n+6],g+=t*s[n+7]}let S=Math.hypot(l,u,d,f)||1;l/=S,u/=S,d/=S,f/=S,p/=S,m/=S,h/=S,g/=S;let C=u*r-d*n+f*t,w=d*t-l*r+f*n,T=l*n-u*t+f*r;return e[0]=t+2*(u*T-d*w)+2*(f*p-g*l+(u*h-d*m)),e[1]=n+2*(d*C-l*T)+2*(f*m-g*u+(d*p-l*h)),e[2]=r+2*(l*w-u*C)+2*(f*h-g*d+(l*m-u*p)),e}function pu(e,t){let n=e[t],r=e[t+1],i=e[t+2],a=e[t+3],o=e[t+4],s=e[t+5],c=e[t+6],l=e[t+7];return[2*(-l*n+o*a-s*i+c*r),2*(-l*r+o*i+s*a-c*n),2*(-l*i-o*r+s*n+c*a)]}const mu=16,hu=20,gu=8,_u=e=>e*2+1,vu=e=>Math.max(0,Math.min(4095,Math.round(e*1e4)+2048));function yu(e){let t=e.positions.length/3;if(e.joints.length<t*4||e.weights.length<t*4)throw new K(`E-DATA-001`,`skinMesh: 4 joints and 4 weights a vertex`,{fix:`joints and weights: 4 numbers for each vertex`});let n=e.normals??bu(e.positions,e.index),r=new Float32Array(t*16),i=e.positions,a=[1/0,1/0,1/0,-1/0,-1/0,-1/0];for(let o=0;o<t;o++){let t=o*16,s=e.joints,c=e.weights;r[t]=i[o*3],r[t+1]=i[o*3+1],r[t+2]=i[o*3+2];for(let e=0;e<3;e++)a[e]=Math.min(a[e],i[o*3+e]),a[e+3]=Math.max(a[e+3],i[o*3+e]);r[t+3]=F(n[o*3],n[o*3+1],n[o*3+2]),r[t+4]=s[o*4]+256*s[o*4+1],r[t+5]=s[o*4+2]+256*s[o*4+3],r[t+6]=e.slots?e.slots[o]:0;let l=e.uvs?e.uvs[o*2]:-1,u=e.uvs?e.uvs[o*2+1]:-1;r[t+7]=l<0?-1:Math.round(Math.min(1,l)*4095)*4096+Math.round(Math.max(0,Math.min(1,u))*4095);let d=c[o*4]+c[o*4+1]+c[o*4+2]+c[o*4+3]||1;for(let e=0;e<4;e++)r[t+8+e]=c[o*4+e]/d;let f=e.morphs,p=f?f.pair[o]:-1;if(f&&p>=0){let e=f.a,n=f.b,i=o*3;r[t+12]=vu(e[i])*4096+vu(e[i+1]),r[t+13]=vu(e[i+2])*4096+vu(n[i]),r[t+14]=vu(n[i+1])*4096+vu(n[i+2]),r[t+15]=p}else r[t+12]=r[t+13]=r[t+14]=8390656,r[t+15]=-1}return{vertices:r,index:Uint32Array.from(e.index),bounds:a}}function bu(e,t){let n=new Float32Array(e.length);for(let r=0;r+2<t.length;r+=3){let i=t[r]*3,a=t[r+1]*3,o=t[r+2]*3,s=e[a]-e[i],c=e[a+1]-e[i+1],l=e[a+2]-e[i+2],u=e[o]-e[i],d=e[o+1]-e[i+1],f=e[o+2]-e[i+2],p=c*f-l*d,m=l*u-s*f,h=s*d-c*u;for(let e of[i,a,o])n[e]=n[e]+p,n[e+1]=n[e+1]+m,n[e+2]=n[e+2]+h}for(let e=0;e<n.length;e+=3){let t=Math.hypot(n[e],n[e+1],n[e+2])||1;n[e]=n[e]/t,n[e+1]=n[e+1]/t,n[e+2]=n[e+2]/t}return n}function xu(e,t,n,r={}){let i=r.loop??!0,a=_u(e.count)*4,o=new Float32Array(t*a);for(let r=0;r<t;r++)Su(e,n(i?r/t:t>1?r/(t-1):0),o,r*a);return{frames:t,loop:i,data:o,distance:r.distance}}function Su(e,t,n,r=0){e.dual(t.pose,t.root??[0,0,0],n,r);let i=t.morphs??[],a=r+e.count*8;for(let e=0;e<4;e++)n[a+e]=i[e]??0}function Cu(e){return{id:e.id,mesh:e.mesh,skeleton:e.skeleton,clips:e.clips??{},material:e.material??null,height:e.height??e.mesh.bounds[4]-e.mesh.bounds[1]}}const wu={id:`plain`,wgsl:``,glsl:``};var Tu=class{g;label;tex=null;width=0;cap=0;used=0;cpu=/* @__PURE__ */ new Float32Array;dirty=null;held=/* @__PURE__ */ new Map;live=/* @__PURE__ */ new Map;spare=[];progs=/* @__PURE__ */ new Map;frameNo=0;drawn={figures:0,instances:0,vertices:0};constructor(e,t=`skin`){this.g=e,this.label=t}hold(e){let t=this.held.get(e);if(t)return t;let n=_u(e.skeleton.count);n>this.width&&this.grow(n,this.cap);let r=/* @__PURE__ */ new Map;for(let[t,n]of Object.entries(e.clips)){let i=this.alloc(n.frames),a=_u(e.skeleton.count)*4;for(let t=0;t<n.frames;t++)this.put(i+t,n.data,t*a,e.skeleton.count);r.set(t,{row:i,frames:n.frames,loop:n.loop})}return t={vbuf:this.g.buffer(`vertex`,e.mesh.vertices,`${this.label} ${e.id}`),ibuf:this.g.buffer(`index`,e.mesh.index,`${this.label} ${e.id} index`),count:e.mesh.index.length,clips:r,inst:null,cap:0},this.held.set(e,t),t}release(e){let t=this.held.get(e);t&&(t.vbuf.destroy(),t.ibuf.destroy(),t.inst?.destroy(),this.held.delete(e))}alloc(e){if(e===1&&this.spare.length)return this.spare.pop();if(this.used+e>this.cap){let t=this.g.caps.maxTex;if(this.used+e>t)throw new K(`E-GPU-003`,`${this.label}: more skin rows than the graphics hold (${t})`,{fix:`fewer figures or shorter clips`});this.grow(this.width,Math.min(t,Math.max(64,(this.used+e)*2)))}let t=this.used;return this.used+=e,t}grow(e,t){let n=this.cpu,r=this.width;this.width=e,this.cap=Math.max(t,64),this.cpu=new Float32Array(this.width*this.cap*4);for(let e=0;e<this.used;e++)this.cpu.set(n.subarray(e*r*4,e*r*4+(r-1)*4),e*this.width*4),this.cpu.set(n.subarray(e*r*4+(r-1)*4,(e+1)*r*4),(e*this.width+this.width-1)*4);this.tex?.destroy(),this.tex=this.g.texture({label:`${this.label} skin rows`,width:this.width,height:this.cap,format:`rgba32f`,filter:`nearest`}),this.dirty=[0,this.cap]}put(e,t,n,r){let i=e*this.width*4;this.cpu.set(t.subarray(n,n+r*8),i),this.cpu.set(t.subarray(n+r*8,n+r*8+4),i+(this.width-1)*4),this.dirty=this.dirty?[Math.min(this.dirty[0],e),Math.max(this.dirty[1],e+1)]:[e,e+1]}scratch=/* @__PURE__ */ new Float32Array;rows(e,t,n){let r=this.hold(e);if(`pose`in t){let r=this.live.get(n);if(r||(r={row:this.alloc(1),seen:this.frameNo,sig:Symbol(`none`)},this.live.set(n,r)),r.seen=this.frameNo,t.key===void 0||t.key!==r.sig){let n=_u(e.skeleton.count)*4;this.scratch.length<n&&(this.scratch=new Float32Array(n)),Su(e.skeleton,t.pose,this.scratch,0),this.put(r.row,this.scratch,0,e.skeleton.count),r.sig=t.key}return[r.row,r.row,0]}let i=(e,t)=>{let n=r.clips.get(e)??r.clips.values().next().value;if(!n)return[0,0,0];let i=n.loop?(t%1+1)%1*n.frames:Math.max(0,Math.min(1,t))*(n.frames-1),a=Math.floor(i);return[n.row+a%n.frames,n.row+(n.loop?(a+1)%n.frames:Math.min(n.frames-1,a+1)),i-a]},a=i(t.clip,t.at);if(!t.to||t.to.mix<=.02)return a;let o=i(t.to.clip,t.to.at);return t.to.mix>=.98?o:[a[2]<.5?a[0]:a[1],o[2]<.5?o[0]:o[1],t.to.mix]}program(e){let t=this.progs.get(e.id);if(t)return t;let n={stride:20,attributes:[0,1,2,3,4].map(e=>({name:`i${e}`,location:e,size:4,offset:e*4}))},r={stride:16,attributes:[0,1,2,3].map(e=>({name:`a${e}`,location:5+e,size:4,offset:e*4}))};return t=this.g.program({label:`${this.label} skin ${e.id}`,wgsl:e.wgsl?ru(e.wgsl):ru(),glsl:e.glsl?iu(e.glsl):iu(),blocks:[`Frame`,`T`],textures:[{name:`ground`,kind:`2d-array`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`tints`,kind:`2d`,sample:`float`},{name:`picture`,kind:`2d`,sample:`float`},{name:`bones`,kind:`2d`,sample:`unfilterable`}],instance:n,vertex:r,depth:{write:!0,test:`less`},cull:`none`}),this.progs.set(e.id,t),t}ready(e){return this.isReady(this.program(e.material??wu))}readyProgs=/* @__PURE__ */ new WeakSet;isReady(e){return this.readyProgs.has(e)?!0:(e.ready.then(()=>this.readyProgs.add(e),()=>{}),!1)}draws(e,t,n,r){if(this.frameNo++,this.frameNo%120==0)for(let[e,t]of this.live)this.frameNo-t.seen>240&&(this.spare.push(t.row),this.live.delete(e));let i=[],a=0,o=0,s=0;for(let e of t)this.hold(e.figure);for(let c of t){if(!c.n)continue;let t=this.hold(c.figure),l=this.program(c.figure.material??wu);if(!this.isReady(l))continue;let u=c.n*20;if(!t.inst||u>t.cap){t.inst?.destroy(),t.cap=Math.max(u,Math.ceil(u*1.5));let e=new Float32Array(t.cap);e.set(c.data.subarray(0,u)),t.inst=this.g.buffer(`instance`,e,`${this.label} ${c.figure.id} instances`)}else t.inst.write(c.data.subarray(0,u));i.push({program:l,blocks:[e.frame,n],textures:[...r,this.tex],instances:t.inst,vertices:t.vbuf,index:t.ibuf,count:t.count,instanceCount:c.n,pickProgram:()=>l,cast:!0,bounds:c.bounds}),a++,o+=c.n,s+=c.figure.mesh.vertices.length/16*c.n}if(this.dirty&&this.tex){let[e,t]=this.dirty,n=Math.min(t,this.cap);n>e&&this.tex.write(this.cpu.subarray(e*this.width*4,n*this.width*4),{y:e,height:n-e,width:this.width}),this.dirty=null}return this.drawn={figures:a,instances:o,vertices:s},i}get bytes(){let e=this.width*this.cap*16;for(let[t,n]of this.held)e+=t.mesh.vertices.byteLength+t.mesh.index.byteLength+n.cap*4;return e}dispose(){for(let e of[...this.held.keys()])this.release(e);this.tex?.destroy(),this.tex=null;for(let e of this.progs.values())e.destroy();this.progs.clear(),this.live.clear()}};const Eu=110574,Du=e=>111320*Math.cos(e*G),Ou=(e,t,n)=>Math.max(t,Math.min(n,e)),ku=()=>({forward:0,right:0,up:0,turn:0,fast:!1,jump:!1}),Au=()=>({kind:`none`,step(){}});function ju(e={}){return e.wheels?Mu({...e.wheels,blocked:e.blocked}):{kind:`surface`,step(t,n,r,i,a){let o=n.fast?e.run??(e.speed??1.4)*3.5:e.speed??1.4,s=Math.sin(a),c=-Math.cos(a),l=-c,u=s,d=s*n.forward+l*n.right,f=c*n.forward+u*n.right,p=Math.hypot(d,f);p>1&&(d/=p,f/=p);let m=Math.min(1,(t.onGround?12:2)*r);t.velocity[0]+=(d*o-t.velocity[0])*m,t.velocity[2]+=(f*o-t.velocity[2])*m,n.jump&&t.onGround&&(t.velocity[1]=e.jump??5.2,t.onGround=!1),t.velocity[1]-=22*r;let h=t.velocity[0]*r,g=t.velocity[2]*r,_=Du(t.at[1]),v=(n,r)=>!e.blocked||e.blocked(t.at[0]+n/_,t.at[1]-r/Eu)<.5;(h||g)&&!v(h,g)&&(v(h,0)?g=0:v(0,g)?h=0:(h=0,g=0));let y=t.at[0]+h/_,b=t.at[1]-g/Eu,x=e.on;if(x&&x!==`walkable`){if(Array.isArray(x))Lu(x,y,b)||(y=t.at[0],b=t.at[1]);else{let e=x.nearest([y,b],{max:50});e&&([y,b]=e.at)}}t.at[0]=y,t.at[1]=b;let S=Fu(i,y,b,t.at[2]);t.at[2]+=t.velocity[1]*r,t.at[2]<=S||t.at[2]-S<.3&&t.velocity[1]<=0?(t.at[2]=S,t.velocity[1]=0,t.onGround=!0):t.onGround=!1;let C=Math.hypot(t.velocity[0],t.velocity[2]);C>.2&&(t.heading=Math.atan2(t.velocity[0],-t.velocity[2])/G),t.speed=C,t.gait+=Math.min(C,8)*r*2.4}}}function Mu(e={}){return{kind:`surface`,step(t,n,r,i){let a=(n.fast?e.boost??100:e.top??60)/3.6,o=n.forward,s=t.speed*Math.sign(t.reverse?-1:1);o>0?s=Math.min(a,s+5*r*o):o<0?s=Math.max(-5,s-(s>0?9:3)*r):s*=Math.exp(-.6*r);let c=t.heading*G+n.right*r*Ou(Math.abs(s)/6,0,1)*1.2*Math.sign(s||1),l=Math.sin(c)*s*r,u=-Math.cos(c)*s*r,d=Du(t.at[1]),f=(n,r)=>!e.blocked||e.blocked(t.at[0]+n/d,t.at[1]-r/Eu)<.5;(l||u)&&!f(l,u)&&(f(l,0)?u=0:f(0,u)?l=0:(l=0,u=0,s*=-.2)),t.at[0]+=l/d,t.at[1]-=u/Eu,t.at[2]=Fu(i,t.at[0],t.at[1],t.at[2]),t.heading=c/G,t.speed=Math.abs(s),t.reverse=s<0,t.onGround=!0}}}function Nu(e={}){return{kind:`air`,step(t,n,r,i,a){let o=(n.fast?e.fast??120:e.speed??40)/3.6,s=Math.sin(a),c=-Math.cos(a),l=-c,u=s,d=e.hover===!1?Math.max(e.glide??.4,n.forward):n.forward,f=s*d+l*n.right,p=c*d+u*n.right,m=Math.hypot(f,p);m>1&&(f/=m,p/=m);let h=Math.min(1,3*r);t.velocity[0]+=(f*o-t.velocity[0])*h,t.velocity[2]+=(p*o-t.velocity[2])*h,t.velocity[1]+=(n.up*o*.6-t.velocity[1])*h;let g=Du(t.at[1]);t.at[0]+=t.velocity[0]*r/g,t.at[1]-=t.velocity[2]*r/Eu;let _=Fu(i,t.at[0],t.at[1]);if(e.over===`sea`){let n=_/(i.exaggeration||1);t.altitude=Ou(t.altitude+t.velocity[1]*r,n+2,Math.max(n+2,e.ceiling??400)),t.at[2]=_+(t.altitude-n)}else t.altitude=Ou(t.altitude+t.velocity[1]*r,2,e.ceiling??400),t.at[2]=_+t.altitude;t.heading=a/G,t.speed=Math.hypot(t.velocity[0],t.velocity[1],t.velocity[2]),t.onGround=!1}}}const Pu={none:()=>Au(),surface:e=>ju(e),air:e=>Nu(e)},Fu=(e,t,n,r=NaN)=>{let i=e.exaggeration,a=e.standAt?e.standAt(t,n,r/i):e.heightAt(t,n);return(Number.isNaN(a)?0:a)*i},Iu=(e,t,n)=>{let r=e.heightAt(t,n);return(Number.isNaN(r)?0:r)*e.exaggeration},Lu=(e,t,n)=>{let r=!1;for(let i=0,a=e.length-1;i<e.length;a=i++){let[o,s]=e[i],[c,l]=e[a];s>n!=l>n&&t<(c-o)*(n-s)/(l-s)+o&&(r=!r)}return r},Ru=(e,...t)=>t.some(t=>e.keys.has(t)),zu={walk(){return{kind:`walk`,help:`W A S D or the stick · Shift run · Space jump · drag to look · Esc map`,read:e=>({...ku(),forward:Ou(+!!Ru(e,`KeyW`,`ArrowUp`)-!!Ru(e,`KeyS`,`ArrowDown`)-e.stick[1],-1,1),right:Ou(+!!Ru(e,`KeyD`,`ArrowRight`)-!!Ru(e,`KeyA`,`ArrowLeft`)+e.stick[0],-1,1),fast:Ru(e,`ShiftLeft`,`ShiftRight`),jump:Ru(e,`Space`)||e.up>0})}},ride(){return{kind:`ride`,help:`W throttle · S brake · A D steer · Shift boost · Esc map`,read:e=>({...ku(),forward:Ou(+!!Ru(e,`KeyW`,`ArrowUp`)-!!Ru(e,`KeyS`,`ArrowDown`)-e.stick[1],-1,1),right:Ou(+!!Ru(e,`KeyD`,`ArrowRight`)-!!Ru(e,`KeyA`,`ArrowLeft`)+e.stick[0],-1,1),fast:Ru(e,`ShiftLeft`,`ShiftRight`)})}},fly(){return{kind:`fly`,help:`W A S D or the stick · E / ▲ up · Q / ▼ down · Shift fast · Esc map`,read:e=>({...ku(),forward:Ou(+!!Ru(e,`KeyW`,`ArrowUp`)-!!Ru(e,`KeyS`,`ArrowDown`)-e.stick[1],-1,1),right:Ou(+!!Ru(e,`KeyD`,`ArrowRight`)-!!Ru(e,`KeyA`,`ArrowLeft`)+e.stick[0],-1,1),up:Ou(+!!Ru(e,`KeyE`,`Space`)-!!Ru(e,`KeyQ`,`KeyC`)+e.up,-1,1),fast:Ru(e,`ShiftLeft`,`ShiftRight`)})}},custom(e,t=``){return{kind:`custom`,help:t,read:t=>({...ku(),...e(t)})}},orbit(e={}){if(e.drag!==void 0&&e.drag!==`pan`&&e.drag!==`orbit`)throw new K(`E-VIEW-001`,`controllers.orbit: drag is ${JSON.stringify(e.drag)}`,{where:`controllers.orbit`,fix:`drag: 'pan' (a one-finger drag moves the map) or 'orbit' (it turns and tilts round the centre)`});return{kind:`orbit`,help:e.drag===`orbit`?`drag to turn and tilt · right-drag or Shift to move · scroll to zoom`:`drag to move · two fingers to turn and tilt · scroll to zoom`,gestures:{pan:e.pan??!0,rotate:e.rotate??!0,tilt:e.tilt??!0,zoom:e.zoom??!0},zoomRange:e.zoomRange,tiltRange:e.tiltRange,drag:e.drag??`pan`}},pan(e={}){return{...zu.orbit({...e,rotate:!1,tilt:!1}),kind:`pan`,help:`drag to move · scroll or pinch to zoom`}}},Bu={player(e){return{kind:`player`,controller:e,want:(t,n)=>e.read(n)}},script(e){return{kind:`script`,want:(t,n,r)=>({...ku(),...e(t,r)})}},data:hc},Vu={auto(e={}){let t=e.run??2.6,n=e.idle??.15,r=/* @__PURE__ */ new WeakMap;return(e,i)=>{let a=i.clips,o=a.walk,s=a.run,c=s?Math.max(0,Math.min(1,(e.speed-t*.8)/(t*.4))):0,l=r.get(e);l||r.set(e,l={walked:e.walked,phase:0});let u=(o?.distance??0)*(1-c)+(s?.distance??o?.distance??0)*c;return l.phase=u>0?l.phase+(e.walked-l.walked)/u:e.gait/(2*Math.PI),l.walked=e.walked,e.speed<n||!o?{clip:a.idle?`idle`:`walk`,at:a.idle?0:l.phase}:c<=0?{clip:`walk`,at:l.phase}:c>=1?{clip:`run`,at:l.phase}:{clip:`walk`,at:l.phase,to:{clip:`run`,at:l.phase,mix:c}}}}},Hu=Vu.auto();var Uu=class e{id;kind;data;placement;footprint;at;heading;velocity=[0,0,0];speed=0;onGround=!0;reverse=!1;altitude=25;gait=0;locomotion;drive;shape;color;body;motion;scale;palette=(/* @__PURE__ */ new Float32Array(8)).fill(16777215);decal=[0,0,0];walked=0;inOpt;layer=null;constructor(e){if(!e?.id)throw new K(`E-DATA-001`,`an entity needs an id`,{fix:`entity({ id: 'bus-12', at: [lon, lat] })`});this.id=e.id,this.kind=e.kind??`thing`,this.data=e.data??{},this.placement=e.placement??`thing`,this.footprint=e.footprint??null,this.at=[e.at[0],e.at[1],e.at[2]??NaN],this.heading=e.heading??0,this.locomotion=e.locomotion??Au(),this.drive=e.drive??null,this.shape=e.shape,this.color=W(e.color??`#ffffff`),this.inOpt=e.in,this.body=e.body,this.motion=e.motion??Hu,this.scale=e.scale??1,e.palette&&this.setPalette(e.palette),e.decal&&(this.decal=[e.decal[0],e.decal[1],e.decal[2]])}setPalette(e){return e.slice(0,8).forEach((e,t)=>this.palette[t]=W(e)),this}get in(){return this.inOpt instanceof e?this.inOpt:this.placement!==`thing`||!this.layer?null:this.layer.list().find(e=>e.placement===`place`&&e.footprint&&Lu(e.footprint,this.at[0],this.at[1]))??null}contents(e={}){return this.placement!==`place`||!this.layer?[]:this.layer.list().filter(t=>t!==this&&t.placement===`thing`&&t.in===this&&(!e.kind||t.kind===e.kind))}joined(e){this.layer=e}};const Wu=e=>new Uu(e);var Gu=class extends Zt{input={keys:/* @__PURE__ */ new Set,stick:[0,0],up:0};look=0;items=/* @__PURE__ */ new Map;offs=[];grounded=/* @__PURE__ */ new WeakSet;standSeen=-1;bodyOpts;waitsFor;framesOpts;constructor(e={}){super({id:e.id??`entities`,hover:!1,fade:0,when:e.when}),this.framesOpts=e.frames??{},this.waitsFor=e.pending??null,this.bodyOpts={pixels:28,budget:32,shadows:!0,...e.bodies}}add(...e){for(let t of e)this.items.set(t.id,t),t.joined(this);return(this.attached?.world)?.invalidate?.(),this}remove(e){let t=this.items.get(e);return t&&(t.joined(null),this.items.delete(e)),this}get(e){return this.items.get(e)}list(){return[...this.items.values()]}control(e,t,n,r){return Yu(e,this,t,n,r)}listen(e){for(let e of this.offs)e();if(this.offs=[],!e||typeof window>`u`)return this;let t=e=>{let t=e.target;t&&/INPUT|SELECT|TEXTAREA/.test(t.tagName)||(this.input.keys.add(e.code),[`Space`,`ArrowUp`,`ArrowDown`].includes(e.code)&&e.preventDefault(),(this.attached?.world)?.invalidate?.())},n=e=>this.input.keys.delete(e.code);return addEventListener(`keydown`,t),addEventListener(`keyup`,n),this.offs.push(()=>{removeEventListener(`keydown`,t),removeEventListener(`keyup`,n),this.input.keys.clear()}),this}frame(e){let t=Math.min(.1,Math.max(0,(e.dt||16)/1e3)),n=e.world,r=n.standVersion??0;for(let e of this.items.values())Number.isNaN(e.at[2])?(e.at[2]=Fu(n,e.at[0],e.at[1]),this.grounded.add(e)):r!==this.standSeen&&!e.drive&&this.grounded.has(e)&&(e.at[2]=Fu(n,e.at[0],e.at[1],e.at[2])),e.drive&&(e.locomotion.step(e,e.drive.want(e,this.input,t),t,n,e.drive.kind===`player`?this.look:e.heading*G),e.walked+=e.speed*t);this.standSeen=r,this.moverRate=this.moversOnScreen(e);let i=this.bodies(e);this.framing=!0;try{this.place(i?.drawn)}finally{this.framing=!1}let a=super.frame(e);return i&&a.push(...i.draws),a}skin=null;skinBlock=null;skinT=/* @__PURE__ */ new Float32Array(16);skinData=/* @__PURE__ */ new Map;bodies(e){let t=this.solidsBase(e);if(!t)return null;let n=e.camera,r=Du(n.lat),i=this.bodyOpts,a=[],o=n.height/(2*Math.tan(n.fov/2)),s=n.eye[0],c=n.eye[1],l=n.eye[2],u=i.pixels/Math.max(.25,Math.min(1,this.detailOf(e.s)));for(let e of this.items.values()){if(!e.body)continue;let t=typeof e.body==`function`?e.body(e):e.body;if(!t)continue;let i=(e.at[0]-n.lon)*r,d=-(e.at[1]-n.lat)*Eu,f=Math.hypot(i-s,e.at[2]+t.height*.5*e.scale-c,d-l);t.height*e.scale*o/Math.max(f,.1)<u||a.push({e,f:t,d:f})}let d=/* @__PURE__ */ new Set;if(!a.length&&!this.skin)return{drawn:d,draws:[]};let f=this.skin??=new Tu(t.g,this.id);a.sort((e,t)=>e.d-t.d);for(let e of this.skinData.values())e.n=0,e.b=[1/0,1/0,1/0,-1/0,-1/0,-1/0];this.bodiesWaiting=0;for(let{e,f:t}of a){if(d.size>=i.budget)break;if(!f.ready(t)){this.bodiesWaiting++;continue}let[a,o,s]=f.rows(t,e.motion(e,t),e),c=this.skinData.get(t);if(c||this.skinData.set(t,c={data:/* @__PURE__ */ new Float32Array(160),n:0,b:[1/0,1/0,1/0,-1/0,-1/0,-1/0]}),(c.n+1)*20>c.data.length){let e=new Float32Array(c.data.length*2);e.set(c.data),c.data=e}let l=(e.at[0]-n.lon)*r,u=-(e.at[1]-n.lat)*Eu,p=c.n*20,m=c.data;m[p]=l,m[p+1]=u,m[p+2]=e.at[2],m[p+3]=e.heading*G+Math.PI,m[p+4]=a,m[p+5]=o,m[p+6]=s,m[p+7]=e.scale,m.set(e.palette,p+8),m[p+16]=d.size,m[p+17]=e.decal[0],m[p+18]=e.decal[1],m[p+19]=e.decal[2],c.n++,d.add(e);let h=t.height*e.scale,g=c.b;g[0]=Math.min(g[0],l-h),g[1]=Math.min(g[1],e.at[2]-.2),g[2]=Math.min(g[2],u-h),g[3]=Math.max(g[3],l+h),g[4]=Math.max(g[4],e.at[2]+h),g[5]=Math.max(g[5],u+h)}let p=n.lat,m=J*Math.cos(p*G),h=this.skinT;h[0]=0,h[1]=0,h[2]=m*G/r,h[3]=m*G/(Eu*Math.cos(p*G)),h[4]=e.world.exaggeration,h[5]=this.pickNo,h[6]=-1,h[7]=1,h[8]=h[9]=h[10]=1,h[11]=255,h[14]=-1,(this.skinBlock??=t.g.buffer(`uniform`,64,`${this.id} bodies block`)).write(h);let g=[...this.skinData].filter(([,e])=>e.n).map(([e,t])=>({figure:e,data:t.data,n:t.n,bounds:t.b.map((e,t)=>t===0||t===3?e*h[2]:t===2||t===5?e*h[3]:e)})),_=f.draws(e,g,this.skinBlock,t.textures);if(!i.shadows)for(let e of _)e.cast=!1;return{drawn:d,draws:_}}bodiesWaiting=0;status(){let e=super.status();return this.bodiesWaiting&&e.pending.push(`${this.bodiesWaiting} bodies waiting for their program`),this.waitsFor&&(this.shown>0||this.target>0)&&e.pending.push(...this.waitsFor()),e.ready=!e.pending.length,e}bodyCounts(){return{...this.skin?.drawn??{figures:0,instances:0,vertices:0},bytes:this.skin?.bytes??0}}animating(){if(super.animating()||this.bodiesWaiting)return!0;for(let e of this.items.values())if(e.drive&&(e.speed>.01||this.input.keys.size||this.input.stick[0]||this.input.stick[1]||this.input.up))return!0;return!1}wantsFrames(){if(this.fading()||this.bodiesWaiting)return 1/0;for(let e of this.items.values())if(e.drive&&(this.input.keys.size||this.input.stick[0]||this.input.stick[1]||this.input.up))return 1/0;return Math.max(this.clockRate(),this.moverRate)}moverRate=0;moversOnScreen(e){let t=e.camera,n=t.viewProj,r=Du(t.lat),i=t.height/(2*Math.tan(t.fov/2)),a=t.eye[0],o=t.eye[1],s=t.eye[2],c=t.morph<.98,l=0;for(let e of this.items.values()){if(!e.drive||e.speed<=.01)continue;let u=(e.at[0]-t.lon)*r,d=-(e.at[1]-t.lat)*Eu,f=Number.isFinite(e.at[2])?e.at[2]:t.groundY,p=Math.max(.5,Math.hypot(u-a,f-o,d-s));if(!c){let e=n[3]*u+n[7]*f+n[11]*d+n[15];if(e<=0)continue;let t=(n[0]*u+n[4]*f+n[8]*d+n[12])/e,r=(n[1]*u+n[5]*f+n[9]*d+n[13])/e;if(Math.abs(t)>1.15||Math.abs(r)>1.15)continue}let m=e.body?typeof e.body==`function`?e.body(e):e.body:null;if((m?.height??1.7)*e.scale*i/p<(this.framesOpts.below??3))continue;let h=m?3:1;l=Math.max(l,e.speed*h*i/p)}return Sn(l,{stepPx:this.framesOpts.step??1})}place(e){let t=[...this.items.values()].filter(t=>t.shape&&!e?.has(t));if(!t.length){for(let e of this.sigs.keys())this.drop(e);this.sigs.clear();return}this.origin??=[t[0].at[0],t[0].at[1]];let n=this.origin,r=Du(n[1]),i=/* @__PURE__ */ new Map;for(let e of t){let t=(e.at[0]-n[0])*r,a=-(e.at[1]-n[1])*Eu,o=Math.floor(t/500),s=Math.floor(a/500),c=`e:${o}_${s}`,l=i.get(c);l||i.set(c,l=/* @__PURE__ */ new Map);let u=typeof e.shape==`function`?e.shape(e):e.shape;if(!u)continue;let d=l.get(u);d||l.set(u,d=[]),d.push(t-o*500,a-s*500,e.at[2],0,1,1,1,e.heading*G+Math.PI,e.color,0)}for(let e of[...this.sigs.keys()])i.has(e)||(this.drop(e),this.sigs.delete(e));for(let[e,t]of i){let r=[...t].map(([e,t])=>({shape:e.levels,data:Float32Array.from(t),m:e})),i=r.map(e=>`${this.shapeNo(e.m)}:${e.data.length}`).join(`,`);if(this.sigs.get(e)!==i){let[t,a]=e.slice(2).split(`_`).map(Number);this.put(e,{origin:n,at:[t*500,a*500],free:!0,instances:r.map(({shape:e,data:t,m:n})=>n.size?{shape:e,data:t,size:n.size,radius:Math.hypot(n.bounds[1][0]-n.bounds[0][0],n.bounds[1][1]-n.bounds[0][1],n.bounds[1][2]-n.bounds[0][2])/2}:{shape:e,data:t}),info:()=>null}),this.sigs.set(e,i)}else r.forEach((t,n)=>this.moveInstances(e,n,t.data))}}origin=null;sigs=/* @__PURE__ */ new Map;shapeNos=/* @__PURE__ */ new WeakMap;nextNo=0;shapeNo(e){let t=this.shapeNos.get(e);return t===void 0&&(t=++this.nextNo,this.shapeNos.set(e,t)),t}detach(){this.listen(!1),this.skin?.dispose(),this.skin=null,this.skinBlock?.destroy(),this.skinBlock=null,this.skinData.clear(),super.detach()}};const Ku=(e={})=>new Gu(e),qu=2*G;function Ju(e,t,n={}){let r=n.offset??[0,3,-8],i=e=>typeof e==`number`&&Number.isFinite(e);if(!(Array.isArray(r)&&r.length===3&&r.every(i)))throw new K(`E-VIEW-001`,`follow: offset is ${JSON.stringify(r)}`,{where:`follow '${t.id}'`,fix:`give offset as three finite metres [right, up, behind] (default [0, 3, -8])`});if(n.lag!==void 0&&!(i(n.lag)&&n.lag>=0))throw new K(`E-VIEW-001`,`follow: lag is ${n.lag}`,{where:`follow '${t.id}'`,fix:`give lag in seconds, a finite number 0 or more, or leave it out`});if(typeof n.eye==`number`&&!i(n.eye))throw new K(`E-VIEW-001`,`follow: eye is ${n.eye}`,{where:`follow '${t.id}'`,fix:`give eye in metres over its foot, a finite number, or leave it out`});let a=null;return e.on(`frame`,({dt:i})=>{let o=n.yaw?n.yaw():t.heading*G,s=Du(t.at[1]),c=Iu(e,t.at[0],t.at[1]),l=Math.hypot(r[1],r[2]),u=n.pitch?n.pitch():l?Math.atan2(r[1],-r[2]):qu,d=typeof n.eye==`function`?n.eye():n.eye??1.3;if(n.blocked&&l>0){let e=-Math.sin(o),r=Math.cos(o);for(let i=1.5;i<l;i+=1){let a=t.at[2]-c+1.3+Math.sin(u)*i;if(n.blocked(t.at[0]+e*Math.cos(u)*i/s,t.at[1]-r*Math.cos(u)*i/Eu)>a-.5){l=Math.max(1.5,i-1);break}}}let f=t.at[2]-c+d,p=Math.max(qu,u),m=f/Math.tan(p)+(n.look===`ahead`?t.speed*.8:0),h=t.at[0]+Math.sin(o)*m/s,g=t.at[1]- -Math.cos(o)*m/Eu,_=n.lag?1-Math.exp(-(i/1e3)/n.lag):1;a=a?[a[0]+(h-a[0])*_,a[1]+(g-a[1])*_]:[h,g],e.go({lon:a[0],lat:a[1],dist:(l+f/Math.sin(p))/e.camera.frameK,tilt:Ou(90-p/G,5,88),bearing:o/G})})}function Yu(e,t,n,r=zu.walk(),i={}){if(!n)return t.listen(!1),()=>{};n.drive=Bu.player(r),t.listen(!0),t.look=n.heading*G;let a=i.eye!==void 0,o=a?[0,0,0]:i.offset??[0,3,-8],s=a?qu:Math.atan2(o[1],-o[2]),c=Ju(e,n,{offset:o,blocked:a?void 0:i.blocked,yaw:()=>t.look,pitch:()=>s,eye:a?i.eye:void 0}),l=e.renderer?.canvas,u=[c];if(l&&typeof l.addEventListener==`function`){let n=null,r=e=>{n||(n={id:e.pointerId,x:e.clientX,y:e.clientY},e.stopPropagation())},i=r=>{n&&r.pointerId===n.id&&(t.look+=(r.clientX-n.x)*.0045,s=Ou(s+(r.clientY-n.y)*.0035,qu,1.3),n.x=r.clientX,n.y=r.clientY,r.stopPropagation(),e.invalidate())},a=e=>{n&&e.pointerId===n.id&&(n=null)};l.addEventListener(`pointerdown`,r,!0),l.addEventListener(`pointermove`,i,!0),l.addEventListener(`pointerup`,a,!0),l.addEventListener(`pointercancel`,a,!0),u.push(()=>{l.removeEventListener(`pointerdown`,r,!0),l.removeEventListener(`pointermove`,i,!0),l.removeEventListener(`pointerup`,a,!0),l.removeEventListener(`pointercancel`,a,!0)})}return e.camera.stop(),()=>{for(let e of u)e();t.listen(!1),n.drive?.kind===`player`&&(n.drive=null)}}async function Xu(e,t){let n=await yo(t.canvas,{body:null,graphics:t.graphics??`auto`,view:{lon:0,lat:0,zoom:19,tilt:60,bearing:330}}),r=nt({id:`preview-${e.id}`,hover:!1,pixels:{full:120,simple:25}});n.add(r);let i=document.createElement(`div`);Object.assign(i.style,{position:`absolute`,left:`8px`,bottom:`8px`,font:`12px system-ui`,color:`#e5e7eb`,background:`rgba(15,23,42,.7)`,padding:`4px 8px`,borderRadius:`6px`,pointerEvents:`none`}),t.canvas.parentElement?.appendChild(i);let a=null,o=[],s=a=>{let s=e.make(a),[c,l]=s.bounds,u=Math.max(.5,l[0]-c[0],l[1]-c[1],l[2]-c[2]),d=u*1.6,f=W(`#ffffff`);o=s.triangles;let p=e=>Float32Array.from([e,250,0,0,1,1,1,Math.PI,f,0].slice(0,10)),m=t.distance?[{shape:s.levels,data:p(250),radius:u/2}]:t.levels??!0?s.levels.map((e,t)=>({shape:[e],data:p(250+(t-1)*d)})):[{shape:s.levels,data:p(250),radius:u/2}];r.put(`shape`,{origin:[0,0],at:[-250,-250],free:!0,instances:m,info:()=>null});let h=e.spec.budget??{};i.textContent=[`whole`,`simple`,`far`].map((e,t)=>`${e} ${o[t]??0}${h[e]===void 0?``:` / ${h[e]}`}`).join(` · `),n.go({lon:0,lat:0,dist:Math.max(4,(t.levels??!0)&&!t.distance?d*2.6:u*3),tilt:62,bearing:330})};return t.distance&&(a=document.createElement(`input`),a.type=`range`,a.min=`1`,a.max=`200`,a.value=`3`,Object.assign(a.style,{position:`absolute`,left:`8px`,top:`8px`,width:`40%`}),a.oninput=()=>{n.go({lon:0,lat:0,dist:Number(a.value)*2,tilt:62,bearing:330})},t.canvas.parentElement?.appendChild(a)),s(t.params??{}),{world:n,set:e=>s({...t.params??{},...e}),get triangles(){return o},dispose(){i.remove(),a?.remove(),n.dispose()}}}const Zu=Math.PI/180;var Qu=class{k=[];v=[];get size(){return this.k.length}push(e,t){let n=this.k,r=this.v,i=n.length;for(n.push(e),r.push(t);i>0;){let t=i-1>>1;if(n[t]<=e)break;n[i]=n[t],r[i]=r[t],i=t}n[i]=e,r[i]=t}pop(){let e=this.k,t=this.v,n=[e[0],t[0]],r=e.pop(),i=t.pop();if(e.length){let n=0;for(;;){let i=n*2+1,a=i+1,o=n,s=r;if(i<e.length&&e[i]<s&&(o=i,s=e[i]),a<e.length&&e[a]<s&&(o=a,s=e[a]),o===n)break;e[n]=e[o],t[n]=t[o],n=o}e[n]=r,t[n]=i}return n}},$u=class{xs=[];zs=[];adj=[];radj;pieces=[];pieceCost=[];pieceWay=[];lines;lon0=0;lat0=0;kx=1;grid=/* @__PURE__ */ new Map;cell=60;constructor(e){if(!Array.isArray(e?.lines))throw new K(`E-DATA-001`,`network needs lines`,{fix:`network({ lines: [{ line: [[lon, lat], …], props }] })`});this.lines=e.lines;let t=0;for(let n of e.lines)for(let e of n.line)this.lon0+=e[0],this.lat0+=e[1],t++;t&&(this.lon0/=t,this.lat0/=t),this.kx=111320*Math.cos(this.lat0*Zu);let n=e.snap??1,r=/* @__PURE__ */ new Map,i=(e,t)=>{let i=`${Math.round(e/n)}_${Math.round(t/n)}`,a=r.get(i);return a===void 0&&(a=this.xs.length,r.set(i,a),this.xs.push(e),this.zs.push(t),this.adj.push([])),a};e.lines.forEach((t,n)=>{let r=t.props??{};if(e.keep&&!e.keep(r))return;let a=e.oneway?e.oneway(r):0,o=a===!0||typeof a==`number`&&a>0?1:typeof a==`number`&&a<0?-1:0,s=-1;for(let a of t.line){let[t,c]=this.local(a),l=i(t,c);if(s>=0&&l!==s){let t=Math.hypot(this.xs[l]-this.xs[s],this.zs[l]-this.zs[s]),i=e.cost?e.cost(t,r):t;o>=0&&this.adj[s].push(l,i),o<=0&&this.adj[l].push(s,i),this.pieces.push(s,l,n),this.pieceCost.push(i),e.oneway&&this.pieceWay.push(o)}s=l}}),this.pieceWay.some(e=>e!==0)?(this.radj=this.adj.map(()=>[]),this.adj.forEach((e,t)=>{for(let n=0;n<e.length;n+=2)this.radj[e[n]].push(t,e[n+1])})):this.radj=this.adj;for(let e=0;e<this.pieces.length;e+=3){let t=this.pieces[e],n=this.pieces[e+1];for(let r=Math.floor(Math.min(this.xs[t],this.xs[n])/this.cell);r<=Math.floor(Math.max(this.xs[t],this.xs[n])/this.cell);r++)for(let i=Math.floor(Math.min(this.zs[t],this.zs[n])/this.cell);i<=Math.floor(Math.max(this.zs[t],this.zs[n])/this.cell);i++){let t=`${r}_${i}`,n=this.grid.get(t);n||this.grid.set(t,n=[]),n.push(e)}}}get size(){return{nodes:this.xs.length,pieces:this.pieces.length/3}}local(e){return[(e[0]-this.lon0)*this.kx,-(e[1]-this.lat0)*110574]}lonLat(e,t){return[this.lon0+e/this.kx,this.lat0-t/110574]}nearest(e,t={}){let[n,r]=this.local(e),i=t.max??300,a=Math.floor(n/this.cell),o=Math.floor(r/this.cell),s=null,c=1/0;for(let e=0;e*this.cell<=i+this.cell;e++){for(let t=a-e;t<=a+e;t++)for(let i=o-e;i<=o+e;i++)if(Math.max(Math.abs(t-a),Math.abs(i-o))===e)for(let e of this.grid.get(`${t}_${i}`)??[]){let t=this.pieces[e],i=this.pieces[e+1],a=this.xs[t],o=this.zs[t],l=this.xs[i]-a,u=this.zs[i]-o,d=l*l+u*u||1,f=Math.max(0,Math.min(1,((n-a)*l+(r-o)*u)/d)),p=a+l*f,m=o+u*f,h=Math.hypot(n-p,r-m);if(h<c){c=h;let n=Math.sqrt(d),r=this.pieceCost[e/3]/Math.max(1e-9,n);s={a:t,b:i,at:this.lonLat(p,m),distance:h,toA:f*n*r,toB:(1-f)*n*r,way:this.pieceWay[e/3]??0}}}if(s&&c<=e*this.cell)break}return s&&c<=i?s:null}shortest(e,t={}){let n=this.xs.length,r=new Float64Array(n).fill(1/0),i=new Int32Array(n).fill(-1),a=new Int32Array(n).fill(-1),o=new Qu,s=(e,t,n)=>{t<r[e]&&(r[e]=t,a[e]=n,i[e]=-1,o.push(t,e))},c=!!t.toward,l=c?this.radj:this.adj;for(e.forEach((e,n)=>{if(`node`in e){s(e.node,e.cost??0,n);return}let r=this.nearest(e.at,{max:t.max});r&&((c?r.way!==-1:r.way!==1)&&s(r.a,(e.cost??0)+r.distance+r.toA,n),(c?r.way!==1:r.way!==-1)&&s(r.b,(e.cost??0)+r.distance+r.toB,n))});o.size;){let[e,t]=o.pop();if(e>r[t])continue;let n=l[t];for(let s=0;s<n.length;s+=2){let c=n[s],l=e+n[s+1];l<r[c]&&(r[c]=l,i[c]=t,a[c]=a[t],o.push(l,c))}}let u=(e,t)=>{let n=this.nearest(e,{max:t});if(!n)return null;let i=(c?n.way!==1:n.way!==-1)?r[n.a]+n.toA:1/0,a=(c?n.way!==-1:n.way!==1)?r[n.b]+n.toB:1/0;return{near:n,node:i<=a?n.a:n.b,cost:n.distance+Math.min(i,a)}};return{dist:r,prev:i,origin:a,costAt:(e,t={})=>{let n=u(e,t.max);return n?n.cost:1/0},pathFrom:(e,t={})=>{let n=u(e,t.max);if(!n||!Number.isFinite(n.cost))return null;let r=[e,n.near.at],a=n.node,o=0;for(;a>=0&&o++<1e5;)r.push(this.lonLat(this.xs[a],this.zs[a])),a=i[a];return r}}}route(e,t,n={}){let r=this.shortest([{at:t}],{...n,toward:!0}),i=r.pathFrom(e,n),a=r.costAt(e,n),o=this.nearest(e,n),s=this.nearest(t,n);if(o&&s&&o.a===s.a&&o.b===s.b&&(s.toA>=o.toA?o.way!==-1:o.way!==1)){let n=o.distance+Math.abs(s.toA-o.toA)+s.distance;if(!(n>=a))return{cost:n,line:[e,o.at,s.at,t]}}return i?(i.push(t),{cost:a,line:i}):null}matrix(e,t,n={}){return e.map(e=>{let r=this.shortest([{at:e}],n);return t.map(e=>r.costAt(e,n))})}isochrone(e,t,n={}){let r;if(ed(e)){if(e.dist.length!==this.xs.length)throw new K(`E-DATA-001`,`isochrone: the pass given has ${e.dist.length} nodes, this network ${this.xs.length}`,{fix:`give a pass made by this network: net.isochrone(net.shortest(sources), [400, 800])`});r=e}else{let t=Array.isArray(e[0])||e.length&&typeof e[0]==`object`?e:[e];r=this.shortest(t.map(e=>Array.isArray(e)?{at:e}:e),n)}let i=[],a=Math.max(...t),o=n.cell??25,s=n.reach??60,c=n.off??1.2,l=1/0,u=-1/0,d=1/0,f=-1/0;for(let e=0;e<this.xs.length;e++)r.dist[e]<=a&&(l=Math.min(l,this.xs[e]),u=Math.max(u,this.xs[e]),d=Math.min(d,this.zs[e]),f=Math.max(f,this.zs[e]));if(!Number.isFinite(l))return i;l-=s+o,d-=s+o,u+=s+o,f+=s+o;let p=Math.ceil((u-l)/o)+1,m=Math.ceil((f-d)/o)+1,h=new Float32Array(p*m).fill(1/0),g=Math.ceil(s/o),_=(e,t,n)=>{let r=Math.round((e-l)/o),i=Math.round((t-d)/o);for(let a=Math.max(0,i-g);a<=Math.min(m-1,i+g);a++)for(let i=Math.max(0,r-g);i<=Math.min(p-1,r+g);i++){let r=Math.hypot(l+i*o-e,d+a*o-t);if(r>s)continue;let u=n+r*c;u<h[a*p+i]&&(h[a*p+i]=u)}};for(let e=0;e<this.pieces.length;e+=3){let t=this.pieces[e],n=this.pieces[e+1],i=r.dist[t],s=r.dist[n];if(!(Math.min(i,s)<=a))continue;let c=Math.hypot(this.xs[n]-this.xs[t],this.zs[n]-this.zs[t]),l=this.pieceCost[e/3]??c,u=Math.max(1,Math.ceil(c/(o*.75)));for(let e=0;e<=u;e++){let r=e/u,o=Math.min(i+r*l,s+(1-r)*l);o<=a&&_(this.xs[t]+(this.xs[n]-this.xs[t])*r,this.zs[t]+(this.zs[n]-this.zs[t])*r,o)}}for(let e=0;e<this.xs.length;e++){let t=r.dist[e];t<=a&&!this.adj[e].length&&!this.radj[e].length&&_(this.xs[e],this.zs[e],t)}for(let e of[...t].sort((e,t)=>t-e)){let t=nd(h,p,m,e).map(e=>e.map(([e,t])=>this.lonLat(l+e*o,d+t*o)));if(!t.length)continue;let n=e=>{let t=0;for(let n=0,r=e.length-1;n<e.length;r=n++)t+=(e[r][0]+e[n][0])*(e[r][1]-e[n][1]);return Math.abs(t/2)},r=(e,t)=>{let n=!1;for(let r=0,i=t.length-1;r<t.length;i=r++){let[a,o]=t[r],[s,c]=t[i];o>e[1]!=c>e[1]&&e[0]<(s-a)*(e[1]-o)/(c-o)+a&&(n=!n)}return n},a=t.filter(e=>e.length>=4).sort((e,t)=>n(t)-n(e)),s=[],c=[];a.forEach((e,t)=>{let i=0,o=-1;for(let s=0;s<t;s++)r(e[0],a[s])&&(i++,(o<0||n(a[s])<n(a[o]))&&(o=s));i%2==1&&o>=0&&c[o]>=0?(s[c[o]].push(e),c[t]=-1):(c[t]=s.length,s.push([e]))}),i.push({id:`within-${e}`,props:{within:e,areas:s.length},geometry:s.length===1?{type:`Polygon`,coordinates:s[0]}:{type:`MultiPolygon`,coordinates:s}})}return i}};const ed=e=>!!e&&typeof e==`object`&&!Array.isArray(e)&&e.dist instanceof Float64Array&&e.prev instanceof Int32Array,td=e=>new $u(e);function nd(e,t,n,r){let i=(r,i)=>r<0||i<0||r>=t||i>=n?1/0:e[i*t+r],a=(e,t)=>!Number.isFinite(e)||!Number.isFinite(t)?.5:Math.max(0,Math.min(1,(r-e)/(t-e))),o=/* @__PURE__ */ new Map,s=(e,t,n)=>{let r=i(e,t),o=i(e+1,t),s=i(e+1,t+1),c=i(e,t+1);return n===0?[`h${e},${t}`,[e+a(r,o),t]]:n===1?[`v${e+1},${t}`,[e+1,t+a(o,s)]]:n===2?[`h${e},${t+1}`,[e+a(c,s),t+1]]:[`v${e},${t}`,[e,t+a(r,c)]]},c=(e,t,n,r)=>{let[i,a]=s(e,t,n),[c,l]=s(e,t,r),u={a:i,b:c,pa:a,pb:l};for(let e of[i,c]){let t=o.get(e);t||o.set(e,t=[]),t.push(u)}};for(let e=-1;e<n;e++)for(let n=-1;n<t;n++)switch(+(i(n,e)<r)|(i(n+1,e)<r?2:0)|(i(n+1,e+1)<r?4:0)|(i(n,e+1)<r?8:0)){case 1:case 14:c(n,e,3,0);break;case 2:case 13:c(n,e,0,1);break;case 3:case 12:c(n,e,3,1);break;case 4:case 11:c(n,e,1,2);break;case 6:case 9:c(n,e,0,2);break;case 7:case 8:c(n,e,3,2);break;case 5:c(n,e,3,0),c(n,e,1,2);break;case 10:c(n,e,0,1),c(n,e,3,2)}let l=/* @__PURE__ */ new Set,u=[];for(let e of o.values())for(let t of e){if(l.has(t))continue;l.add(t);let e=[t.pa,t.pb],n=t.b;for(let t=0;t<1e6;t++){let t=(o.get(n)??[]).find(e=>!l.has(e));if(!t)break;l.add(t),t.a===n?(e.push(t.pb),n=t.b):(e.push(t.pa),n=t.a)}if(e.length>=4){let t=e[0],n=e[e.length-1];(t[0]!==n[0]||t[1]!==n[1])&&e.push(t),u.push(e)}}return u}const rd=5.67e-8,id=273.15,ad=Math.PI/180,od=245e4,sd=.5,cd=2*Math.PI/86400,ld=e=>6.105*Math.exp(17.27*e/(237.7+e)),ud=(e,t)=>t/100*ld(e);function dd(e,t,n){let r=ud(e,t),i=e+id;return Math.min(1,1.24*(r/i)**(1/7)*(1+.22*n*n))*rd*i**4}const fd=(e,t=1)=>t*rd*(e+id)**4,pd=(e,t)=>e*(Math.log(Math.max(t,sd*1.2)/sd)/Math.log(10/sd)),md=e=>6+4*e,hd=`typical published values (Oke et al. 2017, Urban Climates; TEB / SUEWS tables), rounded`,gd={asphalt:{id:`asphalt`,label:`Asphalt road`,albedo:.08,emissivity:.95,evap:0,storage:.35,rhoC:194e4,k:.75,waterCap:1,color:`#2f3236`,source:hd},coolpave:{id:`coolpave`,label:`Light (cool) pavement`,albedo:.33,emissivity:.92,evap:0,storage:.3,rhoC:194e4,k:.75,waterCap:1,color:`#9a9a96`,source:hd},concrete:{id:`concrete`,label:`Concrete footpath`,albedo:.3,emissivity:.92,evap:0,storage:.3,rhoC:2e6,k:1.5,waterCap:1,color:`#c9c5bb`,source:hd},grass:{id:`grass`,label:`Watered grass`,albedo:.23,emissivity:.97,evap:.8,storage:.1,rhoC:25e5,k:1.2,waterCap:10,color:`#5f9a45`,source:hd},drygrass:{id:`drygrass`,label:`Dry grass / soil`,albedo:.2,emissivity:.95,evap:.1,storage:.2,rhoC:13e5,k:.3,waterCap:10,color:`#b8a774`,source:hd},roof:{id:`roof`,label:`Building`,albedo:.15,emissivity:.9,evap:0,storage:.3,rhoC:14e5,k:.7,waterCap:1,color:`#7a4a35`,source:hd},water:{id:`water`,label:`Water`,albedo:.06,emissivity:.98,evap:.9,storage:.6,rhoC:418e4,k:.6,waterCap:1e3,color:`#3f7fae`,source:hd}},_d={id:`brick`,label:`Brick wall`,albedo:.3,emissivity:.9,evap:0,storage:.3,rhoC:14e5,k:.7,waterCap:0,color:`#b0674a`,source:hd},vd=(e,t)=>t?e.albedo*.8:e.albedo,yd=(e,t)=>t?Math.max(e.evap,.7):e.evap;function bd(e,t,n,r,i,a=!1){let o=vd(e,a),s=yd(e,a),c=i+id,l=c;for(let i=0;i<12;i++){let i=(1-o)*t+e.emissivity*n-e.emissivity*rd*l**4,a=i>0?(1-e.storage)*(1-s):1;l-=(i*a-r*(l-c))/(-a*4*e.emissivity*rd*l**3-r)}return l-id}const xd=e=>e.rhoC*Math.sqrt(2*e.k/(e.rhoC*cd))/2;function Sd(e,t,n,r,i,a,o,s,c,l){let u=vd(t,c),d=yd(t,c),f=e+id,p=(1-u)*r+t.emissivity*i-t.emissivity*rd*f**4,m=d*Math.max(0,p);return{T:e+l*((p-a*(e-o)-m)/n-cd*(e-s)),LE:m}}function Cd(e){let t=[-2836.5744,-6028.076559,19.54263612,-.02737830188,16261698e-12,70229056e-17,-18680009e-20],n=e+273.15,r=2.7150305*Math.log(n);for(let e=0;e<t.length;e++)r+=t[e]*n**(e-2);return Math.exp(r)*.01}function wd(e,t,n,r){let i=Math.max(-50,Math.min(50,e)),a=Math.max(.5,Math.min(17,n)),o=Math.max(-30,Math.min(70,t-i)),s=Cd(i)*(r/100)/10;return i+.607562052+-.0227712343*i+8.06470249*10**-4*i*i+-1.54271372*10**-4*i*i*i+-3.24651735*10**-6*i*i*i*i+7.32602852*10**-8*i*i*i*i*i+1.35959073*10**-9*i*i*i*i*i*i+-2.2583652*a+.0880326035*i*a+.00216844454*i*i*a+-1.53347087*10**-5*i*i*i*a+-5.72983704*10**-7*i*i*i*i*a+-2.55090145*10**-9*i*i*i*i*i*a+-.751269505*a*a+-.00408350271*i*a*a+-5.21670675*10**-5*i*i*a*a+1.94544667*10**-6*i*i*i*a*a+1.14099531*10**-8*i*i*i*i*a*a+.158137256*a*a*a+-6.57263143*10**-5*i*a*a*a+2.22697524*10**-7*i*i*a*a*a+-4.16117031*10**-8*i*i*i*a*a*a+-.0127762753*a*a*a*a+9.66891875*10**-6*i*a*a*a*a+2.52785852*10**-9*i*i*a*a*a*a+4.56306672*10**-4*a*a*a*a*a+-1.74202546*10**-7*i*a*a*a*a*a+-5.91491269*10**-6*a*a*a*a*a*a+.398374029*o+1.83945314*10**-4*i*o+-1.7375451*10**-4*i*i*o+-7.60781159*10**-7*i*i*i*o+3.77830287*10**-8*i*i*i*i*o+5.43079673*10**-10*i*i*i*i*i*o+-.0200518269*a*o+8.92859837*10**-4*i*a*o+3.45433048*10**-6*i*i*a*o+-3.77925774*10**-7*i*i*i*a*o+-1.69699377*10**-9*i*i*i*i*a*o+1.69992415*10**-4*a*a*o+-4.99204314*10**-5*i*a*a*o+2.47417178*10**-7*i*i*a*a*o+1.07596466*10**-8*i*i*i*a*a*o+8.49242932*10**-5*a*a*a*o+1.35191328*10**-6*i*a*a*a*o+-6.21531254*10**-9*i*i*a*a*a*o+-4.99410301*10**-6*a*a*a*a*o+-1.89489258*10**-8*i*a*a*a*a*o+8.15300114*10**-8*a*a*a*a*a*o+7.5504309*10**-4*o*o+-5.65095215*10**-5*i*o*o+-4.52166564*10**-7*i*i*o*o+2.46688878*10**-8*i*i*i*o*o+2.42674348*10**-10*i*i*i*i*o*o+1.5454725*10**-4*a*o*o+5.2411097*10**-6*i*a*o*o+-8.75874982*10**-8*i*i*a*o*o+-1.50743064*10**-9*i*i*i*a*o*o+-1.56236307*10**-5*a*a*o*o+-1.33895614*10**-7*i*a*a*o*o+2.49709824*10**-9*i*i*a*a*o*o+6.51711721*10**-7*a*a*a*o*o+1.94960053*10**-9*i*a*a*a*o*o+-1.00361113*10**-8*a*a*a*a*o*o+-1.21206673*10**-5*o*o*o+-2.1820366*10**-7*i*o*o*o+7.51269482*10**-9*i*i*o*o*o+9.79063848*10**-11*i*i*i*o*o*o+1.25006734*10**-6*a*o*o*o+-1.81584736*10**-9*i*a*o*o*o+-3.52197671*10**-10*i*i*a*o*o*o+-3.3651463*10**-8*a*a*o*o*o+1.35908359*10**-10*i*a*a*o*o*o+4.1703262*10**-10*a*a*a*o*o*o+-1.30369025*10**-9*o*o*o*o+4.13908461*10**-10*i*o*o*o*o+9.22652254*10**-12*i*i*o*o*o*o+-5.08220384*10**-9*a*o*o*o*o+-2.24730961*10**-11*i*a*o*o*o*o+1.17139133*10**-10*a*a*o*o*o*o+6.62154879*10**-10*o*o*o*o*o+4.0386326*10**-13*i*o*o*o*o*o+1.95087203*10**-12*a*o*o*o*o*o+-4.73602469*10**-12*o*o*o*o*o*o+5.12733497*s+-.312788561*i*s+-.0196701861*i*i*s+9.9969087*10**-4*i*i*i*s+9.51738512*10**-6*i*i*i*i*s+-4.66426341*10**-7*i*i*i*i*i*s+.548050612*a*s+-.00330552823*i*a*s+-.0016411944*i*i*a*s+-5.16670694*10**-6*i*i*i*a*s+9.52692432*10**-7*i*i*i*i*a*s+-.0429223622*a*a*s+.00500845667*i*a*a*s+1.00601257*10**-6*i*i*a*a*s+-1.81748644*10**-6*i*i*i*a*a*s+-1.25813502*10**-3*a*a*a*s+-1.79330391*10**-4*i*a*a*a*s+2.34994441*10**-6*i*i*a*a*a*s+1.29735808*10**-4*a*a*a*a*s+1.2906487*10**-6*i*a*a*a*a*s+-2.28558686*10**-6*a*a*a*a*a*s+-.0369476348*o*s+.00162325322*i*o*s+-3.1427968*10**-5*i*i*o*s+2.59835559*10**-6*i*i*i*o*s+-4.77136523*10**-8*i*i*i*i*o*s+8.6420339*10**-3*a*o*s+-6.87405181*10**-4*i*a*o*s+-9.13863872*10**-6*i*i*a*o*s+5.15916806*10**-7*i*i*i*a*o*s+-3.59217476*10**-5*a*a*o*s+3.28696511*10**-5*i*a*a*o*s+-7.10542454*10**-7*i*i*a*a*o*s+-1.243823*10**-5*a*a*a*o*s+-7.385844*10**-9*i*a*a*a*o*s+2.20609296*10**-7*a*a*a*a*o*s+-7.3246918*10**-4*o*o*s+-1.87381964*10**-5*i*o*o*s+4.80925239*10**-6*i*i*o*o*s+-8.7549204*10**-8*i*i*i*o*o*s+2.7786293*10**-5*a*o*o*s+-5.06004592*10**-6*i*a*o*o*s+1.14325367*10**-7*i*i*a*o*o*s+2.53016723*10**-6*a*a*o*o*s+-1.72857035*10**-8*i*a*a*o*o*s+-3.95079398*10**-8*a*a*a*o*o*s+-3.59413173*10**-7*o*o*o*s+7.04388046*10**-7*i*o*o*o*s+-1.89309167*10**-8*i*i*o*o*o*s+-4.79768731*10**-7*a*o*o*o*s+7.96079978*10**-9*i*a*o*o*o*s+1.62897058*10**-9*a*a*o*o*o*s+3.94367674*10**-8*o*o*o*o*s+-1.18566247*10**-9*i*o*o*o*o*s+3.34678041*10**-10*a*o*o*o*o*s+-1.15606447*10**-10*o*o*o*o*o*s+-2.80626406*s*s+.548712484*i*s*s+-.0039942841*i*i*s*s+-9.54009191*10**-4*i*i*i*s*s+1.93090978*10**-5*i*i*i*i*s*s+-.308806365*a*s*s+.0116952364*i*a*s*s+4.95271903*10**-4*i*i*a*s*s+-1.90710882*10**-5*i*i*i*a*s*s+.00210787756*a*a*s*s+-6.98445738*10**-4*i*a*a*s*s+2.30109073*10**-5*i*i*a*a*s*s+4.1785659*10**-4*a*a*a*s*s+-1.27043871*10**-5*i*a*a*a*s*s+-3.04620472*10**-6*a*a*a*a*s*s+.0514507424*o*s*s+-.00432510997*i*o*s*s+8.99281156*10**-5*i*i*o*s*s+-7.14663943*10**-7*i*i*i*o*s*s+-2.66016305*10**-4*a*o*s*s+2.63789586*10**-4*i*a*o*s*s+-7.01199003*10**-6*i*i*a*o*s*s+-1.06823306*10**-4*a*a*o*s*s+3.61341136*10**-6*i*a*a*o*s*s+2.29748967*10**-7*a*a*a*o*s*s+3.04788893*10**-4*o*o*s*s+-6.42070836*10**-5*i*o*o*s*s+1.16257971*10**-6*i*i*o*o*s*s+7.68023384*10**-6*a*o*o*s*s+-5.47446896*10**-7*i*a*o*o*s*s+-3.5993791*10**-8*a*a*o*o*s*s+-4.36497725*10**-6*o*o*o*s*s+1.68737969*10**-7*i*o*o*o*s*s+2.67489271*10**-8*a*o*o*o*s*s+3.23926897*10**-9*o*o*o*o*s*s+-.0353874123*s*s*s+-.22120119*i*s*s*s+.0155126038*i*i*s*s*s+-2.63917279*10**-4*i*i*i*s*s*s+.0453433455*a*s*s*s+-.00432943862*i*a*s*s*s+1.45389826*10**-4*i*i*a*s*s*s+2.1750861*10**-4*a*a*s*s*s+-6.66724702*10**-5*i*a*a*s*s*s+3.3321714*10**-5*a*a*a*s*s*s+-.00226921615*o*s*s*s+3.80261982*10**-4*i*o*s*s*s+-5.45314314*10**-9*i*i*o*s*s*s+-7.96355448*10**-4*a*o*s*s*s+2.53458034*10**-5*i*a*o*s*s*s+-6.31223658*10**-6*a*a*o*s*s*s+3.02122035*10**-4*o*o*s*s*s+-4.77403547*10**-6*i*o*o*s*s*s+1.73825715*10**-6*a*o*o*s*s*s+-4.09087898*10**-7*o*o*o*s*s*s+.614155345*s*s*s*s+-.0616755931*i*s*s*s*s+.00133374846*i*i*s*s*s*s+.00355375387*a*s*s*s*s+-5.13027851*10**-4*i*a*s*s*s*s+1.02449757*10**-4*a*a*s*s*s*s+-.00148526421*o*s*s*s*s+-4.11469183*10**-5*i*o*s*s*s*s+-6.80434415*10**-6*a*o*s*s*s*s+-9.77675906*10**-6*o*o*s*s*s*s+.0882773108*s*s*s*s*s+-.00301859306*i*s*s*s*s*s+.00104452989*a*s*s*s*s*s+2.47090539*10**-4*o*s*s*s*s*s+.00148348065*s*s*s*s*s*s}const Td=[[-40,`extreme cold stress`],[-27,`very strong cold stress`],[-13,`strong cold stress`],[0,`moderate cold stress`],[9,`slight cold stress`],[26,`no thermal stress`],[32,`moderate heat stress`],[38,`strong heat stress`],[46,`very strong heat stress`],[1/0,`extreme heat stress`]];function Ed(e){for(let[t,n]of Td)if(e<t)return n;return`extreme heat stress`}const Dd={memory:!0,surroundings:!0,walls:!0,rain:!0,coupling:!0,canyon:!0,canopyTransmission:.03},Od=[{key:`memory`,label:`Heat stored through the day`,help:`Surfaces warm up from the morning and give heat back after sunset`},{key:`surroundings`,label:`Heat from surrounding surfaces`,help:`The road beside you and the grass strip count, not only the ground underfoot`},{key:`walls`,label:`Sun-heated walls`,help:`Each street front's temperature from its sun, sky, ground and air`},{key:`rain`,label:`Rain wets surfaces`,help:`Water on the ground evaporates and cools it; wet surfaces are darker`},{key:`coupling`,label:`Surfaces and street air coupled`,help:`Surfaces lose heat to the warmer street air rather than the weather station's air`},{key:`canyon`,label:`Street air from canyon heat budget`,help:`Air warmed by road, walls and traffic, flushed by wind; deep canyons flush slower`}],kd=.7,Ad=[[0,-1],[1,0],[0,1],[-1,0]];function jd(e){let t=e.w*e.h,n=e.cell,r=Pd(e),i=Fd(e),a=e.regions.length+1,o=new Float32Array(t),s=Array.from({length:a},()=>({area:0,crownArea:0,traffic:!1,height:0,width:25}));for(let r=0;r<t;r++){let t=e.region[r],i=t>0?e.regions[t-1]:void 0;o[r]=(i?.tallBuildings?.7:1)*(e.cTop[r]>0?.8:1),e.walk[r]&&t>0&&(s[t].area+=n*n,e.cTop[r]>0&&(s[t].crownArea+=n*n))}for(let t=1;t<a;t++){let n=e.regions[t-1],r=i.filter(e=>e.region===t),a=r.reduce((e,t)=>e+t.area,0);s[t].traffic=!!n.traffic,s[t].width=n.width??25,s[t].height=a?r.reduce((e,t)=>e+t.h*t.area,0)/a:0}let c={albedo:new Float32Array(t),emissivity:new Float32Array(t),evap:new Float32Array(t),waterCap:new Float32Array(t),capacity:new Float32Array(t),hcFactor:new Float32Array(t),building:new Uint8Array(t)};for(let n=0;n<t;n++){let t=e.materials[e.mat[n]];c.albedo[n]=t.albedo,c.emissivity[n]=t.emissivity,c.evap[n]=t.evap,c.waterCap[n]=t.waterCap,c.capacity[n]=xd(t),c.hcFactor[n]=t.id===`grass`||t.id===`drygrass`?1.5:1,c.building[n]=+(e.bH[n]>0)}let l=/* @__PURE__ */ new Map,u=/* @__PURE__ */ new Map,d=/* @__PURE__ */ new Map,f=xd(_d),p=e=>{let t=l.get(e);return t||(t=Rd(e,n),l.set(e,t)),t},m=(t,r)=>{let i=Math.floor(t/n),a=Math.floor(r/n);return i<0||a<0||i>=e.w||a>=e.h?-1:a*e.w+i},h=t=>e.materials[e.mat[t]];function g(t,n,r){let a=n.elevation*ad,o=`${n.elevation.toFixed(2)}|${n.azimuth.toFixed(2)}|${r.canopyTransmission}`,s=u.get(o);s||(s=Nd(e,n,r.canopyTransmission),u.size>64&&u.clear(),u.set(o,s));let c=Md(e,n,r.canopyTransmission),l=a>.02?Math.min(1100,t.dirH/Math.sin(a)):0,d=i.map(e=>r.walls?Ld(e,n,c):0),f=i.map(e=>Id(e,n));return{Sg:s,vis:c,In:l,Lsky:dd(t.ta,t.rh,t.cloud),faceSun:d,faceInc:f}}function _(e,t,n,r,o){let s=Array.from({length:a},(e,t)=>n.ta+(o[t]??0)+2),c=Array.from({length:a},()=>.3*(n.dirH+n.difH));if(!r.walls)return{T:s,K:c};let l=new Float64Array(a),u=new Float64Array(a),d=new Float64Array(a);i.forEach((n,r)=>{l[n.region]+=e[r]*n.area,u[n.region]+=t[r]*n.area,d[n.region]+=n.area});for(let e=0;e<a;e++)d[e]&&(s[e]=l[e]/d[e],c[e]=u[e]/d[e]);return{T:s,K:c}}function v(t,n,i,a,o){let s=r[t],c=e.region[t];return{K:n.dirH*i[t]+n.difH*s+(1-s)*_d.albedo*o.K[c],Lin:s*a+(1-s)*fd(o.T[c],.95)}}function y(t,n,a,o){let s=i[t],c=0,l=0,u=0;for(let e of s.front){let t=h(e);c+=n.dirH*a.Sg[e]+n.difH*r[e],l+=fd(o[e],t.emissivity),u+=t.albedo}let d=Math.max(1,s.front.length),f=a.In*a.faceInc[t]*a.faceSun[t]+.4*n.difH+u/d*.5*(c/d),p=.5*(.8*a.Lsky+.2*fd(n.ta+2,.95))+.5*(s.front.length?l/d:fd(n.ta,.95)),m=e.regions[s.region-1];return{K:f,Lin:p,hc:md(pd(n.u10,s.h/2)*(m?.tallBuildings?.7:1))}}let b=(e,t)=>md(pd(t.u10,2)*o[e])*c.hcFactor[e];function x(e,t,n,r){if(!e.area)return 0;let i=e.height>0?Math.exp(-e.height/(4*e.width)):1,a=Math.max(.01,.03*pd(n,Math.max(4,e.height))*i),o=t+(e.traffic?50*e.area:0)-.25*r*e.crownArea;return Math.max(-3,Math.min(6,o/(1200*a*e.area)))}let S=(e,t,n)=>Math.max(-2.5,Math.min(3,(.04*e-1.5*t)/(1+.3*n)));function C(e,t,r,i){return s.map((a,o)=>a.area?i.canyon?x(a,e[o],r.u10,r.dirH+r.difH):S(t[o]/(a.area/(n*n)),a.crownArea/a.area,r.u10):0)}function w(r,o,s,c,l){let u=new Float64Array(a),d=new Float64Array(a);for(let i=0;i<t;i++){if(!e.walk[i])continue;let t=e.region[i];u[t]+=b(i,s)*(r[i]-(s.ta+(l[t]??0)))*n*n,d[t]+=r[i]-s.ta}return i.forEach((e,t)=>{u[e.region]+=md(pd(s.u10,e.h/2))*(o[t]-(s.ta+(l[e.region]??0)))*e.area}),C(u,d,s,c)}function T(n,r){let o=n.weather,s=n.sunAt(n.hour),l=g(o,s,r),u=new Float32Array(t),d=new Uint8Array(t),f=new Float32Array(i.length).fill(o.ta+2),p=new Float32Array(i.length),m=r.rain&&(o.precip||0)>.1,x=Array(a).fill(0);for(let n=0;n<(r.coupling?3:1);n++){let n=_(f,p,o,r,x);for(let i=0;i<t;i++){if(c.building[i]){u[i]=o.ta+8;continue}let{K:t,Lin:a}=v(i,o,l.Sg,l.Lsky,n);d[i]=+!!m,u[i]=bd(h(i),t,a,b(i,o),o.ta+(r.coupling?x[e.region[i]]:0),m)}i.forEach((e,t)=>{let{K:n,Lin:i,hc:a}=y(t,o,l,u);p[t]=n,f[t]=r.walls?bd(_d,n,i,a,o.ta+(r.coupling?x[e.region]:0)):o.ta+2}),x=w(u,f,o,r,r.coupling?x:Array(a).fill(0))}return{hour:n.hour,w:o,sun:s,Ts:u,wet:d,Sg:l.Sg,wallT:f,wallK:p,dStreet:x,memoryUsed:!1}}async function E(s,l,u,d){let p=s.hours,m=i.length,h=p.reduce((e,t)=>e+t.ta,0)/p.length,v=new Float32Array(t).fill(p[0].ta),b=new Float32Array(t),x=new Uint8Array(t),S=new Float32Array(t).fill(h),w=new Float64Array(t),T=new Float32Array(m).fill(p[0].ta+2),E=new Float32Array(m),D=new Float32Array(m).fill(h),O=new Float64Array(m),k=new Float32Array(t),A=new Float32Array(t),j=[],M=Array(a).fill(0);for(let s=0;s<2;s++){for(let h of p){let p=l(h.hour),N=g(h,p,u),P=u.rain?(h.precip||0)*600/3600:0,F=pd(h.u10,2);for(let e=0;e<t;e++)k[e]=h.dirH*N.Sg[e]+h.difH*r[e],A[e]=md(F*o[e])*c.hcFactor[e];for(let o=0;o<6;o++){let o=_(T,E,h,u,M),l=o.T.map(e=>fd(e,.95)),d=o.K.map(e=>_d.albedo*e),p=new Float64Array(a),g=new Float64Array(a);for(let i=0;i<t;i++){if(c.building[i]){v[i]=h.ta+8;continue}let t=e.region[i],a=r[i],o=k[i]+(1-a)*d[t],s=a*N.Lsky+(1-a)*l[t],f=u.rain&&b[i]>.05,m=f?c.albedo[i]*.8:c.albedo[i],_=f?Math.max(c.evap[i],.7):c.evap[i],y=v[i],C=y+273.15,w=c.emissivity[i],T=h.ta+(u.coupling?M[t]:0),E=(1-m)*o+w*s-w*5.67e-8*C*C*C*C,D=_*(E>0?E:0),O=A[i]*(y-T);e.walk[i]&&(p[t]+=A[i]*(y-(h.ta+M[t]))*n*n,g[t]+=y-h.ta),v[i]=y+600*((E-O-D)/c.capacity[i]-cd*(y-S[i])),x[i]=+!!f,u.rain&&(b[i]=Math.max(0,Math.min(c.waterCap[i],b[i]+P-(f?D*600/od:0))))}for(let e=0;e<m;e++){let t=i[e],{K:n,Lin:r,hc:a}=y(e,h,N,v);E[e]=n,T[e]=u.walls?Sd(T[e],_d,f,n,r,a,h.ta+(u.coupling?M[t.region]:0),D[e],!1,600).T:h.ta+2,p[t.region]+=md(pd(h.u10,t.h/2))*(T[e]-(h.ta+M[t.region]))*t.area}if(M=C(p,g,h,u),s===0){for(let e=0;e<t;e++)w[e]+=v[e];for(let e=0;e<m;e++)O[e]+=T[e]}}s===1&&(j[h.hour]={hour:h.hour,w:h,sun:p,Ts:v.slice(),wet:x.slice(),Sg:N.Sg,wallT:T.slice(),wallK:E.slice(),dStreet:M.slice(),memoryUsed:!0}),d&&await d()}if(s===0){let e=p.length*6;for(let n=0;n<t;n++)S[n]=w[n]/e;for(let t=0;t<m;t++)D[t]=O[t]/e}}return j}async function D(e,t,n){if(!t.memory||!e.day||e.day.hours.length<24)return T(e,t);let r=`${e.key}|${JSON.stringify(t)}`,i=d.get(r);return i||(d.size>12&&d.clear(),i=await E(e.day,e.sunAt,t,n),d.set(r,i)),i[e.hour]??T(e,t)}async function O(n,a=Dd,s={}){let l=await D(n,a,s.yield),u=l.w,d=l.sun,f=g(u,d,a),y=d.elevation*ad,x=d.azimuth*ad,S=_(l.wallT,l.wallK,u,a,l.dStreet),C=ud(u.ta,u.rh),w=new Float32Array(t),T=new Float32Array(t);for(let n=0;n<t;n++){if(c.building[n]){let t=e.region[n];w[n]=fd(S.T[t]??u.ta+2,_d.emissivity),T[n]=_d.albedo*(S.K[t]??0);continue}let t=h(n),r=v(n,u,l.Sg,f.Lsky,S);T[n]=vd(t,!!l.wet[n])*r.K,w[n]=fd(l.Ts[n],t.emissivity)+(1-t.emissivity)*r.Lin}let E=(e,t)=>{let n=i.findIndex(n=>n.region===e&&n.q===t);return n>=0?{T:l.wallT[n],K:l.wallK[n]}:null},O=t=>bd(h(t),u.dirH*(e.cTrans?.[t]||a.canopyTransmission)+u.difH*r[t],r[t]*f.Lsky+(1-r[t])*fd(S.T[e.region[t]],.95),b(t,u),u.ta+l.dStreet[e.region[t]],!!l.wet[t]);return{Ts:l.Ts,Sg:l.Sg,dStreet:l.dStreet,wallT:l.wallT,Lsky:f.Lsky,In:f.In,memoryUsed:l.memoryUsed,point:(t,n,i,s)=>{let d=m(t,n);if(d<0||c.building[d])return null;let g=d%e.w,_=Math.floor(d/e.w),v=e.region[d],b=h(d),D=s??f.vis(t,n,i),k=r[d],A=s!==void 0&&s<.5?O(d):l.Ts[d],j=.1*(A-u.ta)*Math.exp(-i/.25),M=u.ta+l.dStreet[v]+j,N=fd(a.walls?S.T[v]:M+2,.95),P=e=>f.In*Math.cos(y)*D*Math.max(0,Math.cos(x-e*Math.PI/2)),F,I,L,R,z=0,B=0,V,H=0,U=null;if(a.surroundings){let t=p(i),n=0,r=0,o=0,c=[0,0,0,0],m=[0,0,0,0],h=[0,0,0,0];for(let i=0;i<t.di.length;i++){let a=g+t.di[i],p=_+t.dj[i];if(a<0||p<0||a>=e.w||p>=e.h)continue;let v=p*e.w+a,y=T[v],x=w[v];v===d&&s!==void 0&&(y=vd(b,!!l.wet[d])*(u.dirH*s+u.difH*k),x=fd(A,b.emissivity)+(1-b.emissivity)*f.Lsky),n+=t.down[i]*y,r+=t.down[i]*x,o+=t.down[i];for(let e=0;e<4;e++){let n=t.sides[e][i];n&&(c[e]+=n*y,m[e]+=n*x,h[e]+=n)}}L=n/(o||1),R=r/(o||1),V=R;let y=zd(e,g,_,i),x=0,S=0,C=0,O=0,j=0;y.forEach((e,t)=>{let n=N,r=0,i=M+2;if(e.kind===`wall`){let e=E(v,(t+2)%4);i=a.walls&&e?e.T:M+2,n=fd(i,_d.emissivity),r=e?_d.albedo*e.K:0}else e.kind===`crown`?(i=M+1,n=fd(i,.97),r=.075*(u.dirH+u.difH)):e.kind===`awning`&&(i=M+5,n=fd(i,.9),r=0);let o=c[t]/(h[t]||1)*.5,s=m[t]/(h[t]||1)*.5,l=P(t);B+=l;let d=l+.5*(1-e.sinH)*u.difH+.5*e.sinH*r+o,p=.5*(1-e.sinH)*f.Lsky+.5*e.sinH*n+s;z+=.22*(.7*d+.97*p),x+=e.sinH*n,S+=e.sinH*r,C+=e.sinH,e.kind!==`open`&&e.sinH>.05&&(O+=i*e.sinH,j+=e.sinH)}),H=C?x/C:0,U=j?O/j:null,F=u.dirH*D+u.difH*k+(1-k)*(C?S/C:0),I=k*f.Lsky+(1-k)*(C?x/C:N)}else{let e=(1-k)*.3*.5*(u.dirH+u.difH);F=u.dirH*D+u.difH*k+e*.5,L=vd(b,!!l.wet[d])*(u.dirH*(s??l.Sg[d])+u.difH*k),I=k*f.Lsky+(1-k)*N,R=fd(A,b.emissivity)+(1-b.emissivity)*I;let t=.5*(I+R);for(let n=0;n<4;n++){let r=P(n);B+=r,z+=.22*(.7*(r+.5*(u.difH*k+L+e))+.97*t)}V=R}let ee=((.06*(.7*F+.97*I)+.06*(.7*L+.97*R)+z)/(.97*5.67e-8))**.25-273.15,W=Math.max(.5,u.u10*o[d]),te=.97*5.67e-8*((ee+273.15)**4-(M+273.15)**4),ne=M+.348*C-.7*W+.7*te/(W+10)-4.25,re=wd(M,ee,W,Math.min(100,100*C/ld(M)));return{k:d,material:b,region:v,height:i,sunVisible:D,svf:k,surface:A,airBackground:u.ta,streetAir:l.dStreet[v],nearSurface:j,air:M,Kdown:F,Kup:L,kDirect:B,beam:f.In*D,Ldown:I,Lup:R,Lsky:f.Lsky,groundL:V,besideL:H,besideT:U,wet:!!l.wet[d],tmrt:ee,wind:pd(u.u10,i)*o[d],ws10:W,vapour:C,radiationLoad:te,feels:ne,utci:re,stress:Ed(re),canopyOverhead:e.cTop[d]>0,underAwning:(e.aw?.[d]??0)>0,memoryUsed:l.memoryUsed}},sunVisibility:f.vis}}return{rasters:e,svf:r,facades:i,field:O}}function Md(e,t,n){let r=t.elevation*ad,i=t.azimuth*ad,a=Math.sin(i),o=-Math.cos(i),s=Math.tan(Math.max(r,.001)),c=e.cell,l=0;for(let t=0;t<e.bH.length;t++)e.bH[t]>l&&(l=e.bH[t]),e.cTop[t]>l&&(l=e.cTop[t]);let u=(t,n)=>{let r=Math.floor(t/c),i=Math.floor(n/c);return r<0||i<0||r>=e.w||i>=e.h?-1:i*e.w+r};return(t,i,d)=>{if(r<=0)return 0;if(e.aw&&d<3){let n=(3-d)/s,r=u(t+a*n,i+o*n);if(r>=0&&e.aw[r]>0)return 0}let f=1;for(let r=c*.5;r<400;r+=c*.5){let c=d+r*s;if(c>l)break;let p=u(t+a*r,i+o*r);if(p<0)break;if(e.bH[p]>c)return 0;e.cTop[p]>0&&c>=e.cBase[p]&&c<=e.cTop[p]&&(f=Math.min(f,e.cTrans?.[p]||n))}return f}}function Nd(e,t,n){let r=new Float32Array(e.w*e.h);if(t.elevation<=0)return r;let i=Md(e,t,n),a=e.cell;for(let t=0;t<e.h;t++)for(let n=0;n<e.w;n++){let o=t*e.w+n;e.bH[o]>0||(r[o]=i((n+.5)*a,(t+.5)*a,.05))}return r}function Pd(e){let t=new Float32Array(e.w*e.h),n=e.cell,r=Math.ceil(60/n),i=Array.from({length:16},(e,t)=>[Math.cos(2*Math.PI*t/16),Math.sin(2*Math.PI*t/16)]);for(let a=0;a<e.h;a++)for(let o=0;o<e.w;o++){let s=a*e.w+o;if(e.bH[s]>0)continue;let c=0;for(let[t,l]of i){let i=0,u=0;for(let s=1;s<=r;s++){let r=Math.floor(o+.5+t*s),c=Math.floor(a+.5+l*s);if(r<0||c<0||r>=e.w||c>=e.h)break;let d=c*e.w+r,f=s*n;e.bH[d]>0&&(i=Math.max(i,e.bH[d]/f)),e.cTop[d]>0&&(u=Math.max(u,e.cTop[d]/f)),e.aw&&e.aw[d]>0&&(i=Math.max(i,e.aw[d]/f))}e.cTop[s]>0&&(u=Math.max(u,50)),c+=Math.max(i*i/(1+i*i),kd*(u*u/(1+u*u)))}t[s]=(1-c/16)*(e.aw&&e.aw[s]>0?.25:1)}return t}function Fd(e){let t=/* @__PURE__ */ new Map,n=e.cell;for(let r=0;r<e.h;r++)for(let i=0;i<e.w;i++){let a=r*e.w+i,o=e.bH[a];o<=0||Ad.forEach(([a,s],c)=>{let l=i+a,u=r+s;if(l<0||u<0||l>=e.w||u>=e.h)return;let d=u*e.w+l;if(e.bH[d]>0)return;let f=e.region[d];if(!f)return;let p=`${f}|${c}`,m=t.get(p);m||(m={region:f,q:c,nx:a,nz:s,area:0,h:0,samples:[],front:[]},t.set(p,m)),m.h=(m.h*m.area+o*n*o)/(m.area+n*o),m.area+=n*o,m.front.length<400&&(i+r)%2==0&&m.front.push(d),m.samples.length<72&&(i*7+r*13)%5==0&&m.samples.push((l+.5)*n-a*n*.4,(u+.5)*n-s*n*.4,o)})}return[...t.values()]}const Id=(e,t)=>{if(t.elevation<=0)return 0;let n=t.elevation*ad,r=t.azimuth*ad;return Math.max(0,Math.cos(n)*(Math.sin(r)*e.nx-Math.cos(r)*e.nz))};function Ld(e,t,n){if(Id(e,t)<=0||!e.samples.length)return 0;let r=0,i=0;for(let t=0;t<e.samples.length;t+=3)for(let a of[.2,.5,.8])r+=n(e.samples[t]+e.nx*.6,e.samples[t+1]+e.nz*.6,a*e.samples[t+2]),i++;return r/i}function Rd(e,t){let n=Math.ceil(10/t),r=[],i=[],a=[],o=[[],[],[],[]];for(let s=-n;s<=n;s++)for(let c=-n;c<=n;c++){let n=c*t,l=s*t,u=n*n+l*l;if(u>100)continue;let d=(u+e*e)**2;r.push(c),i.push(s),a.push(e*e/(Math.PI*d)),Ad.forEach(([t,r],i)=>o[i].push(Math.max(0,n*t+l*r)*e/(Math.PI*d)))}let s=(e,t)=>{let n=e.reduce((e,t)=>e+t,0)||1;return Float32Array.from(e,e=>e*t/n)};return{di:Int16Array.from(r),dj:Int16Array.from(i),down:s(a,1),sides:o.map(e=>s(e,.5))}}function zd(e,t,n,r){let i=Math.ceil(40/e.cell),a=e.cell;return Ad.map(([o,s])=>{let c=0,l={sinH:0,kind:`open`};for(let u of[-.52,0,.52]){let d=Math.cos(u),f=Math.sin(u),p=o*d-s*f,m=s*d+o*f,h=0,g=0,_=0;for(let o=1;o<=i;o++){let i=Math.floor(t+.5+p*o),s=Math.floor(n+.5+m*o);if(i<0||s<0||i>=e.w||s>=e.h)break;let c=s*e.w+i,l=o*a;e.bH[c]>r&&(h=Math.max(h,(e.bH[c]-r)/l)),e.cTop[c]>r&&(g=Math.max(g,(e.cTop[c]-r)/l)),e.aw&&e.aw[c]>r&&(_=Math.max(_,(e.aw[c]-r)/l))}let v=e=>e/Math.sqrt(1+e*e),y=v(h),b=kd*v(g),x=v(_),S=Math.max(y,b,x);c+=S,S>l.sinH&&(l={sinH:S,kind:S===y?`wall`:S===b?`crown`:`awning`})}return{...l,sinH:c/3}})}const Bd=`#version 300 es
precision highp float;
precision highp int;
precision highp sampler2D;
`,Vd=`${Bd}
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
`,Hd=e=>`${Bd}
layout(std140) uniform FieldU { vec4 size; vec4 p0; vec4 p1; vec4 p2; vec4 p3; } u;
uniform highp sampler2D cur;
uniform highp sampler2D tin0;
uniform highp sampler2D tin1;
uniform highp sampler2D tin2;
uniform highp sampler2D tin3;
out vec4 outColor;
ivec2 IJ;
vec4 texel(highp sampler2D t, ivec2 p) {
  ivec2 n = textureSize(t, 0);
  return texelFetch(t, clamp(p, ivec2(0, 0), n - ivec2(1, 1)), 0);
}
vec4 at(int dx, int dy) {
  return texel(cur, IJ + ivec2(dx, dy));
}
vec4 in0(int dx, int dy) {
  return texel(tin0, IJ + ivec2(dx, dy));
}
vec4 in1(int dx, int dy) {
  return texel(tin1, IJ + ivec2(dx, dy));
}
vec4 in2(int dx, int dy) {
  return texel(tin2, IJ + ivec2(dx, dy));
}
vec4 in3(int dx, int dy) {
  return texel(tin3, IJ + ivec2(dx, dy));
}
float inside(int dx, int dy) {
  ivec2 p = IJ + ivec2(dx, dy);
  return (p.x >= 0 && p.y >= 0 && p.x < int(u.size.x) && p.y < int(u.size.y)) ? 1.0 : 0.0;
}
float lap() {
  float c = at(0, 0).x;
  return (at(1, 0).x + at(-1, 0).x + at(0, 1).x + at(0, -1).x - 4.0 * c) / (u.size.z * u.size.z);
}
float laplacian(float v) {
  return lap() + 0.0 * v;
}
vec4 sampleAt(vec2 p) {
  vec2 q = clamp(p, vec2(0.0, 0.0), u.size.xy - vec2(1.0, 1.0));
  ivec2 i = ivec2(floor(q));
  vec2 f = q - floor(q);
  vec4 a = texel(cur, i);
  vec4 b = texel(cur, i + ivec2(1, 0));
  vec4 c = texel(cur, i + ivec2(0, 1));
  vec4 d = texel(cur, i + ivec2(1, 1));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
vec4 ruleDiffusion(vec4 c) {
  return vec4(c.x + u.size.w * u.p0.x * lap(), c.yzw);
}
vec4 ruleAdvection(vec4 c) {
  vec2 w = mix(u.p0.xy, in0(0, 0).xy, u.p0.z);
  vec2 back = vec2(float(IJ.x), float(IJ.y)) - vec2(w.x, -w.y) * u.size.w / u.size.z;
  return vec4(sampleAt(back).x, c.yzw);
}
vec4 ruleHeat(vec4 c) {
  float goal = mix(u.p0.z, in0(0, 0).x, u.p0.w);
  float src = in1(0, 0).x * u.p1.x;
  return vec4(c.x + u.size.w * (u.p0.x * lap() + u.p0.y * (goal - c.x) + src), c.yzw);
}
float pipe(vec4 a, vec4 b, float ga, float gb, float f) {
  float dt = u.size.w;
  float cell = u.size.z;
  float face = max(0.0, max(ga + a.x, gb + b.x) - max(ga, gb));
  float q = f * max(0.0, 1.0 - u.p0.y * dt) + dt * u.p0.x * face * cell * ((ga + a.x) - (gb + b.x)) / cell;
  float cap = cell * cell / (4.0 * dt);
  return clamp(q, -b.x * cap, a.x * cap);
}
vec4 ruleWater(vec4 c) {
  float g = in0(0, 0).x;
  float ie = inside(1, 0);
  float is = inside(0, 1);
  float iw = inside(-1, 0);
  float jn = inside(0, -1);
  vec4 e = at(1, 0) * ie;
  vec4 s = at(0, 1) * is;
  vec4 w = at(-1, 0) * iw;
  vec4 n = at(0, -1) * jn;
  float qe = pipe(c, e, g, mix(g, in0(1, 0).x, ie), c.y) * max(ie, u.p0.w);
  float qs = pipe(c, s, g, mix(g, in0(0, 1).x, is), c.z) * max(is, u.p0.w);
  float qw = pipe(w, c, mix(g, in0(-1, 0).x, iw), g, w.y) * max(iw, u.p0.w);
  float qn = pipe(n, c, mix(g, in0(0, -1).x, jn), g, n.z) * max(jn, u.p0.w);
  float rain = u.p0.z + in1(0, 0).x * u.p1.x;
  float h = max(0.0, c.x + u.size.w * ((qw + qn - qe - qs) / (u.size.z * u.size.z) + rain));
  return vec4(h, qe, qs, 0.0);
}
${e}
void main() {
  IJ = ivec2(gl_FragCoord.xy);
  outColor = rule(at(0, 0));
}
`,Ud=e=>`
struct FieldU { size: vec4f, p0: vec4f, p1: vec4f, p2: vec4f, p3: vec4f }
@group(0) @binding(0) var<uniform> u: FieldU;
@group(1) @binding(0) var cur: texture_2d<f32>;
@group(1) @binding(1) var tin0: texture_2d<f32>;
@group(1) @binding(2) var tin1: texture_2d<f32>;
@group(1) @binding(3) var tin2: texture_2d<f32>;
@group(1) @binding(4) var tin3: texture_2d<f32>;
var<private> IJ: vec2i;
// a texel of a grid; past its edge, the nearest one (no flow across the edge)
fn texel(t: texture_2d<f32>, p: vec2i) -> vec4f {
  let n = vec2i(textureDimensions(t, 0));
  return textureLoad(t, clamp(p, vec2i(0, 0), n - vec2i(1, 1)), 0);
}
fn at(dx: i32, dy: i32) -> vec4f {
  return texel(cur, IJ + vec2i(dx, dy));
}
fn in0(dx: i32, dy: i32) -> vec4f {
  return texel(tin0, IJ + vec2i(dx, dy));
}
fn in1(dx: i32, dy: i32) -> vec4f {
  return texel(tin1, IJ + vec2i(dx, dy));
}
fn in2(dx: i32, dy: i32) -> vec4f {
  return texel(tin2, IJ + vec2i(dx, dy));
}
fn in3(dx: i32, dy: i32) -> vec4f {
  return texel(tin3, IJ + vec2i(dx, dy));
}
// 1 where the neighbour is in the grid
fn inside(dx: i32, dy: i32) -> f32 {
  let p = IJ + vec2i(dx, dy);
  return select(0.0, 1.0, p.x >= 0 && p.y >= 0 && p.x < i32(u.size.x) && p.y < i32(u.size.y));
}
// the Laplacian of the first number (per m²)
fn lap() -> f32 {
  let c = at(0, 0).x;
  return (at(1, 0).x + at(-1, 0).x + at(0, 1).x + at(0, -1).x - 4.0 * c) / (u.size.z * u.size.z);
}
fn laplacian(v: f32) -> f32 {
  return lap() + 0.0 * v;
}
// the state between cells (cell centres at whole numbers), bilinear
fn sampleAt(p: vec2f) -> vec4f {
  let q = clamp(p, vec2f(0.0, 0.0), u.size.xy - vec2f(1.0, 1.0));
  let i = vec2i(floor(q));
  let f = q - floor(q);
  let a = texel(cur, i);
  let b = texel(cur, i + vec2i(1, 0));
  let c = texel(cur, i + vec2i(0, 1));
  let d = texel(cur, i + vec2i(1, 1));
  return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}
// diffusion: p0.x the diffusivity (m²/s)
fn ruleDiffusion(c: vec4f) -> vec4f {
  return vec4f(c.x + u.size.w * u.p0.x * lap(), c.yzw);
}
// advection (semi-Lagrangian): the wind p0.xy (m/s east, north), or input 0's xy where p0.z is 1
fn ruleAdvection(c: vec4f) -> vec4f {
  let w = mix(u.p0.xy, in0(0, 0).xy, u.p0.z);
  let back = vec2f(f32(IJ.x), f32(IJ.y)) - vec2f(w.x, -w.y) * u.size.w / u.size.z;
  return vec4f(sampleAt(back).x, c.yzw);
}
// heat: diffusion (p0.x), relaxing (p0.y per s) towards p0.z, or input 0 where p0.w is 1, plus input 1 × p1.x per s
fn ruleHeat(c: vec4f) -> vec4f {
  let goal = mix(u.p0.z, in0(0, 0).x, u.p0.w);
  let src = in1(0, 0).x * u.p1.x;
  return vec4f(c.x + u.size.w * (u.p0.x * lap() + u.p0.y * (goal - c.x) + src), c.yzw);
}
// the pipe model's flux from a to b (m³/s): it was f, damped by friction (p0.y per s), pushed by the difference of
// the water's surface (ground + depth, gravity p0.x) through the depth of water standing over the higher ground at
// the face between them; never more than a quarter of what the giver holds in a step
fn pipe(a: vec4f, b: vec4f, ga: f32, gb: f32, f: f32) -> f32 {
  let dt = u.size.w;
  let cell = u.size.z;
  let face = max(0.0, max(ga + a.x, gb + b.x) - max(ga, gb));
  let q = f * max(0.0, 1.0 - u.p0.y * dt) + dt * u.p0.x * face * cell * ((ga + a.x) - (gb + b.x)) / cell;
  let cap = cell * cell / (4.0 * dt);
  return clamp(q, -b.x * cap, a.x * cap);
}
// shallow water (the pipe model): depth x (m), the flux to the east y and to the south z (m³/s); the ground's height
// is input 0 (m); rain p0.z m/s, plus input 1 × p1.x. Past the grid's edge: dry ground as high as the edge cell, the
// water running out into it where p0.w is 1 (open edges), held where it is 0 (walls)
fn ruleWater(c: vec4f) -> vec4f {
  let g = in0(0, 0).x;
  let ie = inside(1, 0);
  let is = inside(0, 1);
  let iw = inside(-1, 0);
  let jn = inside(0, -1);
  let e = at(1, 0) * ie;
  let s = at(0, 1) * is;
  let w = at(-1, 0) * iw;
  let n = at(0, -1) * jn;
  let qe = pipe(c, e, g, mix(g, in0(1, 0).x, ie), c.y) * max(ie, u.p0.w);
  let qs = pipe(c, s, g, mix(g, in0(0, 1).x, is), c.z) * max(is, u.p0.w);
  let qw = pipe(w, c, mix(g, in0(-1, 0).x, iw), g, w.y) * max(iw, u.p0.w);
  let qn = pipe(n, c, mix(g, in0(0, -1).x, jn), g, n.z) * max(jn, u.p0.w);
  let rain = u.p0.z + in1(0, 0).x * u.p1.x;
  let h = max(0.0, c.x + u.size.w * ((qw + qn - qe - qs) / (u.size.z * u.size.z) + rain));
  return vec4f(h, qe, qs, 0.0);
}
${e}
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  IJ = vec2i(pos.xy);
  return rule(at(0, 0));
}
`,Wd=`${mt}
layout(std140) uniform S { vec4 p; vec4 l; vec4 m; vec4 e; } s;
uniform highp sampler2D state;
uniform highp sampler2D base;
uniform highp sampler2D lut;
${Tn(3)}
ivec2 cellOf(int i, int j) {
  return clamp(ivec2(i, j), ivec2(0, 0), ivec2(int(s.m.y) - 1, int(s.m.z) - 1));
}
float cellValue(int i, int j) {
  vec4 c = texelFetch(state, cellOf(i, j), 0);
  return s.m.w > 2.5 ? c.w : s.m.w > 1.5 ? c.z : s.m.w > 0.5 ? c.y : c.x;
}
float cellTop(int i, int j) {
  return (texelFetch(base, cellOf(i, j), 0).x + cellValue(i, j) * (1.0 - s.e.x)) * s.m.x + s.e.y;
}
`,Gd={vertex:`${Wd}
out vec3 v_world;
out float v_v;
out vec3 v_n;
void main() {
  int cols = int(s.m.y) - 1;
  int vi = gl_VertexID;
  int quad = vi / 6;
  int k = vi - quad * 6;
  int cx = (k == 1 || k == 4 || k == 5) ? 1 : 0;
  int cz = (k == 2 || k == 3 || k == 5) ? 1 : 0;
  int i = quad - (quad / cols) * cols + cx;
  int j = quad / cols + cz;
  float y = cellTop(i, j);
  float gx = (cellTop(i + 1, j) - cellTop(i - 1, j)) / (2.0 * s.p.z);
  float gz = (cellTop(i, j + 1) - cellTop(i, j - 1)) / (2.0 * s.p.w);
  v_world = vec3(s.p.x + float(i) * s.p.z, y, s.p.y + float(j) * s.p.w);
  v_n = normalize(vec3(-gx, 1.0, -gz));
  v_v = cellValue(i, j);
  gl_Position = clipOut(frame.viewProj * vec4(v_world, 1.0));
}
`,fragment:`${Wd}
in vec3 v_world;
in float v_v;
in vec3 v_n;
out vec4 outColor;
void main() {
  if (v_v < s.l.z) {
    discard;
  }
  float t = clamp((v_v - s.l.x) / max(s.l.y - s.l.x, 0.000001), 0.0, 1.0);
  vec4 col = texelFetch(lut, ivec2(int(t * 255.0), 0), 0);
  vec3 c = col.rgb;
  if (frame.sunColor.w > 0.5) {
    c = sunLit(col.rgb, normalize(v_n), v_world);
  }
  vec3 v = normalize(frame.eye.xyz - v_world);
  float fres = pow(1.0 - max(dot(v, normalize(v_n)), 0.0), 3.0);
  c = airFog(mix(themed(c, 0.0), frame.haze.rgb, fres * 0.5), v_world);
  float a = s.l.w * col.a * smoothstep(s.l.z, s.l.z * 3.0 + 0.0001, v_v);
  outColor = vec4(c, a);
}
`},Kd=`${nn}
// p: (the north-west cell's centre in world metres x, z; metres a cell x, z) · l: (the ramp's low and high, the least
// drawn, opacity) · m: (exaggeration, columns, rows, which of the four numbers) · e: (1: lying on its base, the value
// only colouring it; metres above the base, 0, 0)
struct S { p: vec4f, l: vec4f, m: vec4f, e: vec4f }
@group(0) @binding(1) var<uniform> s: S;
@group(1) @binding(0) var state: texture_2d<f32>;
@group(1) @binding(1) var base: texture_2d<f32>;
@group(1) @binding(2) var lut: texture_2d<f32>;
${rt(3)}
struct O { @builtin(position) pos: vec4f, @location(0) world: vec3f, @location(1) v: f32, @location(2) n: vec3f }
fn cellOf(i: i32, j: i32) -> vec2i {
  return clamp(vec2i(i, j), vec2i(0, 0), vec2i(i32(s.m.y) - 1, i32(s.m.z) - 1));
}
fn cellValue(i: i32, j: i32) -> f32 {
  let c = textureLoad(state, cellOf(i, j), 0);
  return select(select(select(c.x, c.y, s.m.w > 0.5), c.z, s.m.w > 1.5), c.w, s.m.w > 2.5);
}
// the top of a cell in world metres (its base plus its value, exaggerated as the ground is)
fn cellTop(i: i32, j: i32) -> f32 {
  return (textureLoad(base, cellOf(i, j), 0).x + cellValue(i, j) * (1.0 - s.e.x)) * s.m.x + s.e.y;
}
@vertex fn vs(@builtin(vertex_index) vi: u32) -> O {
  let cols = u32(s.m.y) - 1u;
  let quad = vi / 6u;
  let k = vi % 6u;
  // a quad's two triangles: (0, 0) (1, 0) (0, 1) · (0, 1) (1, 0) (1, 1)
  let cx = select(0, 1, k == 1u || k == 4u || k == 5u);
  let cz = select(0, 1, k == 2u || k == 3u || k == 5u);
  let i = i32(quad % cols) + cx;
  let j = i32(quad / cols) + cz;
  let y = cellTop(i, j);
  let gx = (cellTop(i + 1, j) - cellTop(i - 1, j)) / (2.0 * s.p.z);
  let gz = (cellTop(i, j + 1) - cellTop(i, j - 1)) / (2.0 * s.p.w);
  var o: O;
  o.world = vec3f(s.p.x + f32(i) * s.p.z, y, s.p.y + f32(j) * s.p.w);
  o.n = normalize(vec3f(-gx, 1.0, -gz));
  o.v = cellValue(i, j);
  o.pos = clipOut(frame.viewProj * vec4f(o.world, 1.0));
  return o;
}
@fragment fn fs(o: O) -> @location(0) vec4f {
  if (o.v < s.l.z) {
    discard;
  }
  let t = clamp((o.v - s.l.x) / max(s.l.y - s.l.x, 0.000001), 0.0, 1.0);
  let col = textureLoad(lut, vec2i(i32(t * 255.0), 0), 0);
  var c = col.rgb;
  if (frame.sunColor.w > 0.5) {
    c = sunLit(col.rgb, o.n, o.world);
  }
  let v = normalize(frame.eye.xyz - o.world);
  let fres = pow(1.0 - max(dot(v, o.n), 0.0), 3.0);
  c = airFog(mix(themed(c, 0.0), frame.haze.rgb, fres * 0.5), o.world);
  let a = s.l.w * col.a * smoothstep(s.l.z, s.l.z * 3.0 + 0.0001, o.v);
  return vec4f(c, a);
}
`,qd=(e,t,n,r,i,a)=>e[(Math.max(0,Math.min(n-1,i))*t+Math.max(0,Math.min(t-1,r))<<2)+a],Jd=(e,t,n)=>+(t>=0&&n>=0&&t<e.w&&n<e.h),Yd=(e,t,n)=>{let r=qd(e.cur,e.w,e.h,t,n,0);return(qd(e.cur,e.w,e.h,t+1,n,0)+qd(e.cur,e.w,e.h,t-1,n,0)+qd(e.cur,e.w,e.h,t,n+1,0)+qd(e.cur,e.w,e.h,t,n-1,0)-4*r)/(e.cell*e.cell)},Xd=(e,t,n,r,i,a)=>{let o=n*e.w+t<<2;r[i]=a,r[i+1]=e.cur[o+1],r[i+2]=e.cur[o+2],r[i+3]=e.cur[o+3]},Zd=(e,t,n,r)=>{let i=Math.max(0,Math.min(e.w-1,t)),a=Math.max(0,Math.min(e.h-1,n)),o=Math.floor(i),s=Math.floor(a),c=i-o,l=a-s,u=qd(e.cur,e.w,e.h,o,s,r),d=qd(e.cur,e.w,e.h,o+1,s,r),f=qd(e.cur,e.w,e.h,o,s+1,r),p=qd(e.cur,e.w,e.h,o+1,s+1,r);return u+(d-u)*c+(f+(p-f)*c-(u+(d-u)*c))*l},Qd=(e,t,n)=>e+(t-e)*n,$d=(e,t)=>e.inputs[t]??null,ef=(e,t,n,r,i)=>{let a=$d(e,t);return a?qd(a,e.w,e.h,n,r,i):0},tf=(e,t,n={})=>{let r={};return e.forEach((e,i)=>{e&&(r[e]={slot:i,value:n[e]??t[e]??0})}),r},nf=(e,t,n,r,i,a,o)=>({name:e,params:n,inputs:r,channels:i,cpu:a,maxDt:o,wgsl:`fn rule(c: vec4f) -> vec4f { return ${t}(c); }`,glsl:`vec4 rule(vec4 c) { return ${t}(c); }`}),rf={diffusion(e={}){return nf(`diffusion`,`ruleDiffusion`,tf([`k`],{k:.2},e),[],[`value`],(e,t,n,r,i)=>Xd(e,t,n,r,i,qd(e.cur,e.w,e.h,t,n,0)+e.dt*e.p[0]*Yd(e,t,n)),(e,t)=>.2*e*e/Math.max(1e-9,t[0]))},advection(e={}){let t=Array.isArray(e.wind)?e.wind:[0,0],n=+(e.wind===`input`);return nf(`advection`,`ruleAdvection`,tf([`windEast`,`windNorth`,`windFromInput`],{windEast:t[0],windNorth:t[1],windFromInput:n}),[`wind`],[`value`],(e,t,n,r,i)=>{let a=Qd(e.p[0],ef(e,0,t,n,0),e.p[2]),o=Qd(e.p[1],ef(e,0,t,n,1),e.p[2]);Xd(e,t,n,r,i,Zd(e,t-a*e.dt/e.cell,n+o*e.dt/e.cell,0))},(e,t)=>.9*e/Math.max(1e-6,Math.hypot(t[0],t[1]),t[2]>.5?30:0))},heat(e={}){let t=+(e.to===`input`),n=typeof e.to==`number`?e.to:0;return nf(`heat`,`ruleHeat`,tf([`k`,`relax`,`to`,`toFromInput`,`sourceScale`],{k:.1,relax:0,to:n,toFromInput:t,sourceScale:1},{...e.k===void 0?{}:{k:e.k},...e.relax===void 0?{}:{relax:e.relax},...e.sourceScale===void 0?{}:{sourceScale:e.sourceScale}}),[`target`,`source`],[`temperature`],(e,t,n,r,i)=>{let a=qd(e.cur,e.w,e.h,t,n,0),o=Qd(e.p[2],ef(e,0,t,n,0),e.p[3]),s=ef(e,1,t,n,0)*e.p[4];Xd(e,t,n,r,i,a+e.dt*(e.p[0]*Yd(e,t,n)+e.p[1]*(o-a)+s))},(e,t)=>Math.min(.2*e*e/Math.max(1e-9,t[0]),t[1]>0?.05/t[1]:1/0))},shallowWater(e={}){return af(`shallowWater`,{friction:.03,gravity:9.81,rain:0,rainScale:1,openEdges:!1,...e})},flow(e={}){return af(`flow`,{friction:.5,gravity:9.81,rain:0,rainScale:1,openEdges:!0,...e})},custom(e,t={}){if(typeof e!=`function`)throw new K(`E-DATA-001`,`rules.custom needs a function (cur, n, dt, p) => next`,{fix:`rules.custom((c, n, dt, p) => c + dt * p.k * n.laplacian, { params: { k: 0.2 } })`});let n=Object.keys(t.params??{}).slice(0,16),r=(t.inputs??[]).slice(0,4),i={};return{name:`custom`,inputs:r,channels:[`value`],params:tf(n,t.params??{}),cpu(t,a,o,s,c){n.forEach((e,n)=>{i[e]=t.p[n]});let l={};r.forEach((e,n)=>{l[e]=ef(t,n,a,o,0)});let u=[qd(t.cur,t.w,t.h,a+1,o,0),qd(t.cur,t.w,t.h,a,o+1,0),qd(t.cur,t.w,t.h,a-1,o,0),qd(t.cur,t.w,t.h,a,o-1,0)];Xd(t,a,o,s,c,e(qd(t.cur,t.w,t.h,a,o,0),{laplacian:Yd(t,a,o),neighbours:u,inputs:l},t.dt,i))},maxDt:()=>t.maxDt??1}}};function af(e,t){let n=tf([`gravity`,`friction`,`rain`,`openEdges`,`rainScale`],{gravity:t.gravity,friction:t.friction,rain:t.rain,openEdges:+!!t.openEdges,rainScale:t.rainScale}),r=(e,t,n,r,i,a)=>{let o=Math.max(0,Math.max(i+t,a+n)-Math.max(i,a)),s=r*Math.max(0,1-e.p[1]*e.dt)+e.dt*e.p[0]*o*e.cell*(i+t-(a+n))/e.cell,c=e.cell*e.cell/(4*e.dt);return Math.max(-n*c,Math.min(t*c,s))};return nf(e,`ruleWater`,n,[`ground`,`rain`],[`depth`,`flux east`,`flux south`,``],(e,t,n,i,a)=>{let o=e.w,s=e.h,c=e.cur,l=e.p[3],u=(t,n)=>ef(e,0,t,n,0),d=(e,t,n)=>qd(c,o,s,e,t,n),f=d(t,n,0),p=u(t,n),m=Jd(e,t+1,n),h=Jd(e,t,n+1),g=Jd(e,t-1,n),_=Jd(e,t,n-1),v=r(e,f,d(t+1,n,0)*m,d(t,n,1),p,Qd(p,u(t+1,n),m))*Math.max(m,l),y=r(e,f,d(t,n+1,0)*h,d(t,n,2),p,Qd(p,u(t,n+1),h))*Math.max(h,l),b=r(e,d(t-1,n,0)*g,f,d(t-1,n,1)*g,Qd(p,u(t-1,n),g),p)*Math.max(g,l),x=r(e,d(t,n-1,0)*_,f,d(t,n-1,2)*_,Qd(p,u(t,n-1),_),p)*Math.max(_,l),S=e.p[2]+ef(e,1,t,n,0)*e.p[4];i[a]=Math.max(0,f+e.dt*((b+x-v-y)/(e.cell*e.cell)+S)),i[a+1]=v,i[a+2]=y,i[a+3]=0},(e,t)=>.25*e/Math.sqrt(Math.max(1e-6,t[0])))}function of(e){if(typeof e?.wgsl!=`string`)throw new K(`E-DATA-001`,`a kernel needs its WGSL body (it sets next)`,{fix:`{ wgsl: 'next = cur + dt * k * laplacian(cur);', params: { k: 0.2 } }`});let t=Object.keys(e.params??{}).slice(0,16),n=e=>`u.p${e>>2}.${`xyzw`[e&3]}`,r=e.cpu?rf.custom(e.cpu,{params:e.params,inputs:e.inputs,maxDt:e.maxDt}):null;return{name:`kernel`,inputs:(e.inputs??[]).slice(0,4),channels:[`value`],params:tf(t,e.params??{}),wgsl:`fn rule(c: vec4f) -> vec4f {\n  let cur = c.x;\n  let dt = u.size.w;\n  let cell = u.size.z;\n${t.map((e,t)=>`  let ${e} = ${n(t)};\n`).join(``)}  var next = cur;\n  ${e.wgsl}\n  return vec4f(next, c.yzw);\n}`,glsl:`vec4 rule(vec4 c) {\n  float cur = c.x;\n  float dt = u.size.w;\n  float cell = u.size.z;\n${t.map((e,t)=>`  float ${e} = ${n(t)};\n`).join(``)}  float next = cur;\n  ${e.glsl??e.wgsl}\n  return vec4(next, c.yzw);\n}`,cpu:r?r.cpu:()=>{throw new K(`E-GPU-003`,`this kernel has no CPU path, and the chip cannot step it here (no float targets)`,{fix:`give the kernel a cpu: (cur, n, dt, p) => next as well`})},maxDt:()=>e.maxDt??1}}function sf(e){if(typeof e==`function`)return rf.custom(e);if(e&&typeof e.cpu==`function`&&`channels`in e)return e;if(e&&typeof e.wgsl==`string`)return of(e);throw new K(`E-DATA-001`,`a field needs a rule: rules.diffusion(…), a kernel { wgsl, cpu } or a function`,{fix:`field({ …, rule: rules.diffusion({ k: 0.2 }) })`})}const cf=Math.PI/180,lf=110574;var uf=class e extends D{kind=`raster`;w;h;cell;box;rule;time=0;version=0;state;next;p=/* @__PURE__ */ new Float32Array(16);inputs=[];dLon;dLat;probes=/* @__PURE__ */ new Set;playing=0;gpu=null;reading=!1;readP=null;readAt=0;dirtyGpu=!1;stepsSinceRead=0;terrainSeen=-1;terrainAt=0;mode=`cpu`;surf=null;constructor(e){if(super(e),!(e.res>0))throw new K(`E-DATA-001`,`field '${this.id}' needs res (metres a cell)`,{where:this.id,fix:`field({ id: 'flood', res: 4, area, rule })`});this.rule=sf(e.rule);let t=e.area,n;if(Array.isArray(e.area))n=e.area;else if(t?.centre&&t.size>0){let e=111320*Math.cos(t.centre[1]*cf),r=t.size/2/e,i=t.size/2/lf;n=[t.centre[0]-r,t.centre[1]-i,t.centre[0]+r,t.centre[1]+i]}else throw new K(`E-DATA-001`,`field '${this.id}' needs an area: [w, s, e, n] or { centre, size }`,{where:this.id,fix:`area: { centre: [145.12, -37.92], size: 800 }`});this.box=n;let r=(n[1]+n[3])/2,i=111320*Math.cos(r*cf);if(this.cell=e.res,this.w=Math.max(2,Math.round((n[2]-n[0])*i/e.res)),this.h=Math.max(2,Math.round((n[3]-n[1])*lf/e.res)),this.w>2048||this.h>2048)throw new K(`E-DATA-001`,`field '${this.id}' would be ${this.w} × ${this.h} cells (at most 2048 a side)`,{where:this.id,fix:`a coarser res or a smaller area`});this.dLon=(n[2]-n[0])/this.w,this.dLat=(n[3]-n[1])/this.h,this.state=new Float32Array(this.w*this.h*4),this.next=new Float32Array(this.state.length),this.setParams(e.params??{}),this.reset(),this.playing=e.rate??0}lonLatOf(e,t){return[this.box[0]+(e+.5)*this.dLon,this.box[3]-(t+.5)*this.dLat]}cellOf(e,t){let n=Math.floor((e-this.box[0])/this.dLon),r=Math.floor((this.box[3]-t)/this.dLat);return n<0||r<0||n>=this.w||r>=this.h?null:[n,r]}get runsOn(){return this.mode}async setRule(e,t={}){let n=sf(e),r=this.gpu?.g??null,i=null;return r&&n.wgsl&&n.glsl&&(i=r.program({label:`field ${this.id}`,wgsl:Ud(n.wgsl),glsl:{vertex:Vd,fragment:Hd(n.glsl)},blocks:[`FieldU`],textures:[`cur`,`tin0`,`tin1`,`tin2`,`tin3`].map(e=>({name:e,kind:`2d`,sample:`unfilterable`})),overlay:!0,format:`rgba32f`,blend:`none`}),await i.ready),this.gpu&&(await this.sync(),i?(this.gpu.prog.destroy(),this.gpu.prog=i,this.mode=`gpu`):this.mode=`cpu`),this.rule=n,this.p.fill(0),this.setParams(t),this.refreshInputs(!0),this.gpu&&this.mode===`gpu`&&this.gpu.a.write(this.state),this.version++,this.invalidate(),this}setParams(e){for(let[t,n]of Object.entries(this.rule.params))this.p[n.slot]=e[t]??n.value;for(let t of Object.keys(e))if(!(t in this.rule.params))throw new K(`E-DATA-001`,`field '${this.id}': the rule '${this.rule.name}' has no number '${t}' (it has ${Object.keys(this.rule.params).join(`, `)||`none`})`,{where:this.id});return this}reset(e=this.props.init){let t=this.state,n=this.w*this.h;if(t.fill(0),typeof e==`number`)for(let r=0;r<n;r++)t[r*4]=e;else if(typeof e==`function`)for(let n=0;n<this.h;n++)for(let r=0;r<this.w;r++){let[i,a]=this.lonLatOf(r,n);t[(n*this.w+r)*4]=e(i,a)}else if(e instanceof Float32Array){if(e.length===n*4)t.set(e);else if(e.length===n)for(let r=0;r<n;r++)t[r*4]=e[r];else throw new K(`E-DATA-001`,`field '${this.id}': init has ${e.length} numbers (want ${n} or ${n*4})`,{where:this.id})}return this.time=0,this.version++,this.gpu&&this.gpu.a.write(t),this.touchProbes(!0),this.invalidate(),this}play(e=this.props.rate||1){return this.playing=e,this.invalidate(),this}pause(){return this.playing=0,this}get rate(){return this.playing}get dt(){return this.props.dt??Math.max(1e-4,Math.min(60,this.rule.maxDt(this.cell,this.p)))}step(e=this.dt){if(!(e>0))return this;let t=Math.max(1,Math.ceil(e/this.dt-1e-9)),n=e/t;return this.refreshInputs(),this.gpu&&this.mode===`gpu`?this.stepGpu(t,n):this.stepCpu(t,n),this.time+=t*n,this.invalidate(),this}async sync(){if(this.mode===`gpu`){for(;this.reading&&this.readP;)await this.readP;this.dirtyGpu&&await this.readBack()}}at(e,t=0){let n=(e[0]-this.box[0])/this.dLon-.5,r=(this.box[3]-e[1])/this.dLat-.5;if(n<-.5||r<-.5||n>this.w-.5||r>this.h-.5)return NaN;let i=Math.max(0,Math.min(this.w-1,n)),a=Math.max(0,Math.min(this.h-1,r)),o=Math.min(this.w-2,Math.floor(i)),s=Math.min(this.h-2,Math.floor(a)),c=i-o,l=a-s,u=this.state,d=this.w,f=(e,n)=>u[(n*d+e)*4+t];return(f(o,s)*(1-c)+f(o+1,s)*c)*(1-l)+(f(o,s+1)*(1-c)+f(o+1,s+1)*c)*l}values(e=0){let t=new Float32Array(this.w*this.h);for(let n=0;n<t.length;n++)t[n]=this.state[n*4+e];return t}probe(e){let t=[];if(e.at){let n=this.cellOf(e.at[0],e.at[1]);n&&t.push(n[1]*this.w+n[0])}else if(e.along)for(let n=0;n+1<e.along.length;n++){let[r,i]=[e.along[n],e.along[n+1]],a=Math.hypot((i[0]-r[0])/this.dLon,(i[1]-r[1])/this.dLat),o=Math.max(1,Math.ceil(a));for(let e=0;e<=o;e++){let n=this.cellOf(r[0]+(i[0]-r[0])*e/o,r[1]+(i[1]-r[1])*e/o);n&&t.push(n[1]*this.w+n[0])}}else if(e.over){let n=e.over;for(let e=0;e<this.h;e++)for(let r=0;r<this.w;r++){let[i,a]=this.lonLatOf(r,e),o=!1;for(let e=0,t=n.length-1;e<n.length;t=e++){let[r,s]=n[e],[c,l]=n[t];s>a!=l>a&&i<(c-r)*(a-s)/(l-s)+r&&(o=!o)}o&&t.push(e*this.w+r)}}else throw new K(`E-DATA-001`,`field '${this.id}': a probe needs at, along or over`,{where:this.id,fix:`field.probe({ at: [lon, lat] })`});let n={cells:[...new Set(t)],series:[],value:NaN,every:e.every??`step`,last:-1/0,on:!0};this.probes.add(n),this.measure(n,!0);let r=this;return{get value(){return n.value},get series(){return n.series},stop(){n.on=!1,r.probes.delete(n)}}}async attach(e){await super.attach(e);let t=this.props.gpu??`auto`,n=e.graphics,r=!!n&&n.caps.floatTargets&&!!this.rule.wgsl&&!!this.rule.glsl;if(t===!0&&!r)throw new K(`E-GPU-003`,`field '${this.id}' was asked to run on the chip, which cannot here (${n?n.caps.floatTargets?`the rule has no kernel`:`no float targets`:`no graphics`})`,{where:this.id,fix:`gpu: 'auto' (the CPU where the chip cannot)`});if(this.refreshInputs(!0),n&&r&&t!==!1)try{let e=n.program({label:`field ${this.id}`,wgsl:Ud(this.rule.wgsl),glsl:{vertex:Vd,fragment:Hd(this.rule.glsl)},blocks:[`FieldU`],textures:[`cur`,`tin0`,`tin1`,`tin2`,`tin3`].map(e=>({name:e,kind:`2d`,sample:`unfilterable`})),overlay:!0,format:`rgba32f`,blend:`none`});await e.ready;let t=e=>n.texture({label:e,width:this.w,height:this.h,format:`rgba32f`,filter:`nearest`,renderable:!0}),r=t(`${this.id} a`),i=t(`${this.id} b`);r.write(this.state);let a=n.texture({label:`${this.id} none`,width:1,height:1,format:`rgba32f`,filter:`nearest`});a.write(/* @__PURE__ */ new Float32Array(4)),this.gpu={g:n,prog:e,a:r,b:i,ta:n.layerTarget(r,0),tb:n.layerTarget(i,0),block:n.buffer(`uniform`,80,`${this.id} field`),ins:[],zero:a},this.uploadInputs(),this.mode=`gpu`}catch(t){this.mode=`cpu`,this.gpu=null,e.world.error(t)}if(n&&this.surfaceDraw)try{await this.attachSurface(n)}catch(t){e.world.error(t)}}get order(){return this.props.order??(this.surfaceDraw?4.5:void 0)}get surfaceDraw(){let e=this.props.draw;return!!e&&e.on===`surface`}async attachSurface(e){let t=e.program({label:`field ${this.id} surface`,wgsl:Kd,glsl:Gd,blocks:[`Frame`,`S`],textures:[{name:`state`,kind:`2d`,sample:`unfilterable`},{name:`base`,kind:`2d`,sample:`unfilterable`},{name:`lut`,kind:`2d`,sample:`float`},{name:`shadowMap`,kind:`2d`,sample:`float`}],blend:`alpha`,depth:{write:!1,test:`less`},cull:`none`});await t.ready;let n=this.props.draw||{},r=this.lut(n),i=e.texture({label:`${this.id} ramp`,width:256,height:1,format:`rgba8`,filter:`nearest`});i.write(new Uint8Array(r.buffer.slice(0)));let a=e.texture({label:`${this.id} base`,width:this.w,height:this.h,format:`rgba32f`,filter:`nearest`});this.surf={g:e,prog:t,block:e.buffer(`uniform`,64,`${this.id} surface`),lut:i,state:null,base:a,baseOf:null,stateVersion:-1,u:/* @__PURE__ */ new Float32Array(16),lutKey:JSON.stringify(n.ramp??`viridis`)},this.writeBase()}writeBase(){let e=this.surf;if(!e)return;let t=(this.props.draw||{}).base??`ground`,n=this.rule.inputs.indexOf(t),r=n>=0?this.inputs[n]:void 0;if(!r){let e=this.attached?.world;r=new Float32Array(this.w*this.h*4);for(let t=0;t<this.h;t++)for(let n=0;n<this.w;n++){let[i,a]=this.lonLatOf(n,t),o=e?e.heightAt(i,a):NaN;r[(t*this.w+n)*4]=Number.isFinite(o)?o:0}}e.baseOf=r,e.base.write(r)}surfaceCall(e){let t=this.surf;if(!t||this.shown<=0)return null;let n;this.gpu&&this.mode===`gpu`?n=this.gpu.a:(t.state||=t.g.texture({label:`${this.id} state`,width:this.w,height:this.h,format:`rgba32f`,filter:`nearest`}),t.stateVersion!==this.version&&(t.state.write(this.state),t.stateVersion=this.version),n=t.state);let r=this.props.draw||{},i=e.camera,a=J*Math.cos(i.lat*cf),o=q(i.lat),[s,c]=this.lonLatOf(0,0),l=(this.box[1]+this.box[3])/2,u=t.u,[d,f]=r.domain??[0,1];u[0]=a*((s-i.lon+540)%360-180)*cf,u[1]=-a*(q(c)-o),u[2]=a*this.dLon*cf,u[3]=a*this.dLat*cf/Math.cos(l*cf),u[4]=d,u[5]=f,u[6]=r.min??-1e30,u[7]=(r.opacity??.8)*this.shown,u[8]=e.world.exaggeration,u[9]=this.w,u[10]=this.h,u[11]=r.channel??0,u[12]=+!!r.flat,u[13]=r.lift??(r.flat?.3:0),u[14]=0,u[15]=0;let p=JSON.stringify(r.ramp??`viridis`);return p!==t.lutKey&&(t.lut.write(new Uint8Array(this.lut(r).buffer.slice(0))),t.lutKey=p),t.block.write(u),{program:t.prog,blocks:[e.frame,t.block],textures:[n,t.base,t.lut,e.shadowMap],count:(this.w-1)*(this.h-1)*6}}detach(){let e=this.surf;this.surf=null,e&&(e.prog.destroy(),e.block.destroy(),e.lut.destroy(),e.base.destroy(),e.state?.destroy());let t=this.gpu;if(this.gpu=null,t){t.a.destroy(),t.b.destroy(),t.ta.destroy(),t.tb.destroy(),t.block.destroy(),t.zero.destroy();for(let e of t.ins)e.destroy();t.prog.destroy()}super.detach()}animating(e){return this.playing>0||this.reading}frame(e){if(this.playing>0&&e.dt>0){let t=Math.min(.25,e.dt/1e3)*this.playing;this.mode===`cpu`&&(t=Math.min(t,Math.max(1,Math.floor(6/Math.max(.01,this.cpuStepMs)))*this.dt)),this.step(t)}this.usesTerrain()&&e.world.groundVersion!==this.terrainSeen&&e.now-this.terrainAt>1e3&&(this.terrainAt=e.now,this.refreshInputs(!0)),this.mode===`gpu`&&this.dirtyGpu&&!this.reading&&e.now-this.readAt>(this.playing>0?250:0)&&this.readBack();let t=this.surfaceDraw?this.surfaceCall(e):null;return t?[t]:[]}painted(){let e=this.props.draw;if(e===!1||this.surfaceDraw)return null;let t=Math.round(this.shown*4)/4;if(t<=0)return null;let n=this.lut(e??{}),[r,i]=e?.domain??[0,1],a=e?.channel??0,o=e?.min??-1/0,s=this;return{kind:`imagery`,id:`${this.id}:field`,attribution:``,terms:Di,where:this.box,minZoom:0,maxZoom:24,opacity:(e?.opacity??.8)*t,blend:`normal`,version:this.version,async draw(e,t){let[c,l,u,d]=le(t.z,t.x,t.y);if(u<s.box[0]||c>s.box[2]||d<s.box[1]||l>s.box[3])return!1;let f=new ImageData(64,64),p=f.data,m=!1;for(let e=0;e<64;e++){let l=(e+.5)/64,d=df(t.z,t.y+l);for(let t=0;t<64;t++){let l=c+(u-c)*(t+.5)/64,f=s.at([l,d],a);if(!Number.isFinite(f)||f<o)continue;let h=Math.max(0,Math.min(255,Math.round((f-r)/(i-r||1)*255)))*4,g=(e*64+t)*4;p[g]=n[h],p[g+1]=n[h+1],p[g+2]=n[h+2],p[g+3]=n[h+3],m=!0}}if(!m)return!1;let h=new OffscreenCanvas(64,64);return h.getContext(`2d`).putImageData(f,0,0),e.imageSmoothingEnabled=!0,e.drawImage(h,0,0,256,256),!0}}}status(){let e=super.status();return this.mode===`gpu`&&this.dirtyGpu&&!this.playing&&e.pending.push(`reading the field back`),e}readouts(){return{time:()=>this.time,runsOn:()=>this.mode,cells:()=>this.w*this.h}}grid(e){return{w:this.w,h:this.h,cell:this.cell,dt:e,cur:this.state,inputs:this.inputs,p:this.p}}cpuStepMs=1;stepCpu(e,t){let n=performance.now();for(let n=0;n<e;n++){let e=this.grid(t),n=this.next;for(let t=0;t<this.h;t++)for(let r=0;r<this.w;r++)this.rule.cpu(e,r,t,n,(t*this.w+r)*4);this.next=this.state,this.state=n}this.cpuStepMs+=((performance.now()-n)/e-this.cpuStepMs)*.3,this.version++,this.touchProbes()}stepGpu(e,t){let n=this.gpu;n.block.write(Float32Array.from([this.w,this.h,this.cell,t,...this.p]));for(let t=0;t<e;t++){let e=[0,1,2,3].map(e=>this.inputTexture(e));n.g.frame([{program:n.prog,blocks:[n.block],textures:[n.a,...e],count:3}],{target:n.tb,clear:[0,0,0,0]}),[n.a,n.b]=[n.b,n.a],[n.ta,n.tb]=[n.tb,n.ta]}this.dirtyGpu=!0,this.stepsSinceRead+=e}readBack(){return!this.gpu||this.reading?this.readP??Promise.resolve():(this.readP=this.readOnce(),this.readP)}async readOnce(){let e=this.gpu;this.reading=!0,this.dirtyGpu=!1,this.readAt=performance.now();try{let t=await e.g.readTexture(e.a);this.gpu===e&&t instanceof Float32Array&&t.length===this.state.length&&(this.state.set(t),this.version++,this.touchProbes())}catch(e){this.attached?.world.error(e)}finally{this.reading=!1,this.invalidate()}}inputTexture(t){let n=this.gpu,r=this.rule.inputs[t],i=r?this.props.inputs?.[r]:void 0;return i instanceof e&&i.gpu&&i.w===this.w&&i.h===this.h?i.gpu.a:n.ins[t]??n.zero}usesTerrain(){return Object.values(this.props.inputs??{}).some(e=>e===`terrain`)}refreshInputs(t=!1){if(!t&&this.inputs.length===this.rule.inputs.length&&!this.rule.inputs.some(t=>this.props.inputs?.[t]instanceof e))return;let n=this.attached?.world;this.inputs=this.rule.inputs.map(t=>{let r=this.props.inputs?.[t],i=new Float32Array(this.w*this.h*4);if(r===void 0)return i;for(let t=0;t<this.h;t++)for(let a=0;a<this.w;a++){let[o,s]=this.lonLatOf(a,t),c=(t*this.w+a)*4;if(typeof r==`number`)i[c]=r;else if(r===`terrain`){let e=n?n.heightAt(o,s):NaN;i[c]=Number.isFinite(e)?e:0}else if(typeof r==`function`)i[c]=r(o,s);else if(r instanceof Float32Array)i[c]=r.length===this.w*this.h*4?r[c]:r[t*this.w+a];else if(r instanceof e)for(let e=0;e<4;e++)i[c+e]=r.at([o,s],e)||0}return i}),n&&(this.terrainSeen=n.groundVersion),this.gpu&&this.uploadInputs(),this.surf&&this.writeBase()}uploadInputs(){let e=this.gpu;this.inputs.forEach((t,n)=>{e.ins[n]||(e.ins[n]=e.g.texture({label:`${this.id} in${n}`,width:this.w,height:this.h,format:`rgba32f`,filter:`nearest`})),e.ins[n].write(t)})}touchProbes(e=!1){for(let t of this.probes)this.measure(t,e)}measure(e,t){if(!e.on)return;let n=0,r=0;for(let t of e.cells){let e=this.state[t*4];Number.isFinite(e)&&(n+=e,r++)}e.value=r?n/r:NaN,(t||e.every===`step`||this.time-e.last>=e.every)&&(e.series.push([this.time,e.value]),e.last=this.time,e.series.length>1e4&&e.series.splice(0,e.series.length-1e4))}luts=/* @__PURE__ */ new Map;lut(e){let t=JSON.stringify(e.ramp??`viridis`),n=this.luts.get(t);if(!n){let r=ve(e.ramp??`viridis`);n=/* @__PURE__ */ new Uint8ClampedArray(1024);for(let e=0;e<256;e++){let t=L(r(e/255));n[e*4]=t[0]*255,n[e*4+1]=t[1]*255,n[e*4+2]=t[2]*255,n[e*4+3]=t[3]*255}this.luts.set(t,n)}return n}invalidate(){(this.attached?.world)?.invalidate?.()}};const df=(e,t)=>{let n=Math.PI-2*Math.PI*t/2**e;return Math.atan(Math.sinh(n))/cf};function ff(e){if(e.kind&&e.kind!==`raster`)throw new K(`E-DATA-001`,`field '${e.id}': kind '${e.kind}' is not here yet (raster is)`,{where:e.id,fix:`kind: 'raster'`});return new uf(e)}const pf={utci:{label:`UTCI (feels like, thermal stress)`,unit:`°C`,range:[26,52]},feels:{label:`Apparent temperature`,unit:`°C`,range:[26,52]},tmrt:{label:`Mean radiant temperature`,unit:`°C`,range:[20,80]},air:{label:`Air temperature`,unit:`°C`,range:[30,46]},surface:{label:`Surface temperature`,unit:`°C`,range:[20,75]}},mf=[`#2c7bb6`,`#00a6ca`,`#90eb9d`,`#f9d057`,`#f29e2e`,`#e76818`,`#d7191c`,`#7a0177`],hf=[`#2166ac`,`#67a9cf`,`#d1e5f0`,`#f7f7f7`,`#fddbc7`,`#ef8a62`,`#b2182b`],gf=()=>new Promise(e=>setTimeout(e,0)),_f=(e,t)=>t===`utci`?e.utci:t===`feels`?e.feels:t===`tmrt`?e.tmrt:t===`air`?e.air:e.surface;var vf=class extends uf{rasters;metric;input;options;summary=null;changed=null;busy=``;model;hf=null;runId=0;hp;constructor(e){let t=e.rasters;if(!t||typeof t.lonLat!=`function`)throw new K(`E-DATA-001`,`heat '${e.id}' needs rasters placed on the Earth (with lonLat and xzOf)`,{where:e.id,fix:`heat({ rasters: cityRasters(street.tiles(), { centre, size: 360 }), input })`});let n=t.lonLat(0,0),r=t.lonLat(t.w*t.cell,t.h*t.cell),i=e.metric??`utci`;super({...e,res:t.cell,area:[n[0],r[1],r[0],n[1]],rule:rf.heat({k:0}),init:NaN,gpu:!1,draw:e.draw!==!1&&{ramp:mf,domain:pf[i].range,opacity:.9,...e.draw||{}}}),this.hp=e,this.rasters=t,this.metric=i,this.input=e.input,this.options={...Dd,...e.options},this.model=jd(t)}async run(e={}){e.input&&(this.input=e.input),e.options&&(this.options={...this.options,...e.options}),e.metric&&(this.metric=e.metric);let t=++this.runId;this.busy=`sun, sky, surfaces through the day…`;let n=await this.model.field(this.input,this.options,{yield:gf});if(t!==this.runId)return null;this.hf=n,this.busy=`what a person feels, cell by cell…`;let{grid:r,summary:i}=await this.sample(n,this.rasters);return t===this.runId?(this.summary=i,this.changed=null,this.busy=``,this.setLook(pf[this.metric].range,mf),this.reset(r),i):null}async compare(e,t={}){this.hf||await this.run();let n=this.runId,r=this.hf;this.busy=`the change…`;let i=await jd(e).field({...this.input,key:`${this.input.key}|variant`},this.options,{yield:gf});if(n!==this.runId)return null;let a=await this.sample(r,this.rasters),o=await this.sample(i,e);if(n!==this.runId)return null;let s=new Float32Array(a.grid.length);for(let e=0;e<s.length;e++)s[e]=o.grid[e]-a.grid[e];return this.summary=a.summary,this.changed=o.summary,this.busy=``,this.setLook(t.domain??[-6,6],hf),this.reset(s),o.summary}point(e){let t=this.hf;if(!t)return null;let[n,r]=this.rasters.xzOf(e[0],e[1]),i=this.hp.height??1.1,a=t.point(n,r,i);return a?{p:a,shade:a.sunVisible>.5?t.point(n,r,i,this.options.canopyTransmission):null}:null}get modelField(){return this.hf}setLook(e,t){let n=this.props.draw||{};this.props.draw!==!1&&this.setProps({draw:{...n,domain:e,ramp:t}})}async sample(e,t){let n=new Float32Array(this.w*this.h).fill(NaN),r=this.hp.height??1.1,i={utci:0,feels:0,tmrt:0,air:0,shaded:0,road:0,points:0},a=0;for(let o=0;o<this.h;o++){for(let s=0;s<this.w;s++){let[c,l]=this.lonLatOf(s,o),[u,d]=t.xzOf(c,l),f=Math.floor(u/t.cell),p=Math.floor(d/t.cell);if(f<0||p<0||f>=t.w||p>=t.h)continue;let m=p*t.w+f;if(t.bH[m]>0)continue;let h=e.point((f+.5)*t.cell,(p+.5)*t.cell,r);h&&(n[o*this.w+s]=_f(h,this.metric),t.walk[m]&&(i.utci+=h.utci,i.feels+=h.feels,i.tmrt+=h.tmrt,i.air+=h.air,h.sunVisible<.5&&i.shaded++,i.points++),(h.material.id===`asphalt`||h.material.id===`coolpave`)&&(i.road+=h.surface,a++))}o%12==0&&await gf()}let o=Math.max(1,i.points);return{grid:n,summary:{utci:i.utci/o,feels:i.feels/o,tmrt:i.tmrt/o,air:i.air/o,shaded:i.shaded/o,road:a?i.road/a:NaN,points:i.points}}}readouts(){return{...super.readouts(),metric:()=>this.metric,utci:()=>this.summary?.utci??null,busy:()=>this.busy}}};const yf=e=>new vf(e),bf={albedo:[``,`the share of sunlight reflected`],emissivity:[``,`how well it radiates heat (1: a black body)`],evap:[``,`the share of its available energy that goes into evaporation`],storage:[``,`the share stored into the ground (the quick balance)`],rhoC:[`J/m³K`,`volumetric heat capacity`],k:[`W/mK`,`thermal conductivity`],waterCap:[`mm`,`water it holds after rain`]},xf=e=>{let t={};for(let[n,[r,i]]of Object.entries(bf))t[n]={value:e[n],unit:r,label:i,source:e.source??`typical published values (Oke et al. 2017, Urban Climates; TEB / SUEWS tables), rounded`};return t},Sf=/* @__PURE__ */ new Map;Sf.set(`materials`,Object.fromEntries([...Object.values(gd),_d].map(e=>[e.id,xf(e)]))),Sf.set(`physics`,{sigma:{value:{value:rd,unit:`W/m²K⁴`,label:`the Stefan–Boltzmann constant`,source:`CODATA 2018`}},kelvin:{value:{value:id,unit:`K`,label:`0 °C in kelvin`,source:`SI`}},rhoCp:{value:{value:1200,unit:`J/m³K`,label:`air: density × specific heat near the ground`,source:`Oke et al. 2017`}},lambda:{value:{value:od,unit:`J/kg`,label:`latent heat of vaporisation at 20 °C`,source:`Oke et al. 2017`}},z0:{value:{value:sd,unit:`m`,label:`a suburb's roughness length`,source:`Grimmond & Oke 1999`}},gravity:{value:{value:9.80665,unit:`m/s²`,label:`standard gravity`,source:`CGPM 1901`}}});const Cf={load(e){let t=Sf.get(e);if(!t)throw new K(`E-DATA-001`,`no constants table '${e}' (there are ${[...Sf.keys()].join(`, `)})`,{fix:`constants.load('materials')`});return t},get(e){let[t,n,r=`value`]=e.split(`.`),i=t&&n?Cf.load(t)[n]?.[r]:void 0;if(!i)throw new K(`E-DATA-001`,`no constant '${e}'`,{fix:`constants.get('materials.asphalt.albedo')`});return i.value},names(){return[...Sf.keys()]},add(e,t){for(let[n,r]of Object.entries(t))for(let[t,i]of Object.entries(r))if(typeof i?.value!=`number`||typeof i.unit!=`string`||typeof i.source!=`string`)throw new K(`E-DATA-001`,`constants '${e}.${n}.${t}' needs { value, unit, source }`,{fix:`{ albedo: { value: 0.1, unit: '', source: 'my measurements' } }`});Sf.set(e,{...Sf.get(e)??{},...t})},materials(e){let t=Cf.load(`materials`);return e.map(e=>{let n=gd[e]??(e===_d.id?_d:null),r=t[e];if(!r)throw new K(`E-DATA-001`,`no material '${e}' in the constants`,{fix:`one of ${Object.keys(t).join(`, `)}`});let i={...n??{id:e,label:e,color:`#888888`,albedo:0,emissivity:0,evap:0,storage:0,rhoC:0,k:0,waterCap:0}};for(let e of Object.keys(bf))r[e]&&(i[e]=r[e].value);return i.source=r.albedo?.source??i.source,i})}},wf=`${mt}
layout(std140) uniform U { mat4 inv; vec4 space; vec4 glow; vec4 horizon; vec4 zenith; vec4 misc; } u;
`,Tf={vertex:`${wf}
out vec2 v_ndc;
void main() {
  int i = gl_VertexID;
  vec2 p = vec2(float((i << 1u) & 2), float(i & 2)) * 2.0 - 1.0;
  gl_Position = clipOut(vec4(p, 1.0, 1.0));
  v_ndc = p;
}
`,fragment:`${wf}
in vec2 v_ndc;
out vec4 outColor;
uint pcg(uint v) {
  uint s = v * 747796405u + 2891336453u;
  uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}
float hash3(vec3 q) {
  ivec3 iq = ivec3(q);
  return float(pcg(uint(iq.x) ^ pcg(uint(iq.y) ^ pcg(uint(iq.z))))) / 4294967295.0;
}
vec3 skyColor(vec2 ndc) {
  vec4 a = u.inv * vec4(ndc, 0.0, 1.0);
  vec4 b = u.inv * vec4(ndc, 1.0, 1.0);
  vec3 o = a.xyz / a.w;
  vec3 d = normalize(b.xyz / b.w - o);
  vec3 q = floor(d * 900.0);
  float s = step(1.0 - 0.0018 * u.misc.z, hash3(q)) * hash3(q + vec3(1.0, 2.0, 3.0));
  vec3 c = u.space.rgb + vec3(s);
  vec3 C = vec3(0.0, -u.misc.y, 0.0);
  vec3 oc = C - o;
  float t = dot(oc, d);
  if (t > 0.0) {
    float dm = length(oc - d * t);
    float g = exp(-max(dm - u.misc.y, 0.0) / (u.misc.y * 0.035)) * smoothstep(u.misc.y * 0.96, u.misc.y, dm);
    c = mix(c, u.glow.rgb, clamp(g, 0.0, 1.0));
  }
  vec3 sky = mix(u.horizon.rgb, u.zenith.rgb, smoothstep(0.0, 0.45, d.y));
  return mix(c, sky, u.misc.x);
}
void main() {
  outColor = vec4(themed(skyColor(v_ndc), 0.0), 1.0);
}
`},Ef=`${nn}
// inv: the inverse view-projection · misc: (morph, radius, stars, 0)
struct U { inv: mat4x4f, space: vec4f, glow: vec4f, horizon: vec4f, zenith: vec4f, misc: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct O { @builtin(position) pos: vec4f, @location(0) ndc: vec2f }
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let i = i32(vid);
  let p = vec2f(f32((i << 1u) & 2), f32(i & 2)) * 2.0 - 1.0;
  var o: O;
  o.pos = clipOut(vec4f(p, 1.0, 1.0));
  o.ndc = p;
  return o;
}
// an integer hash: the same bits on every chip and both graphics
fn pcg(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}
fn hash3(q: vec3f) -> f32 {
  let iq = vec3i(q);
  return f32(pcg(u32(iq.x) ^ pcg(u32(iq.y) ^ pcg(u32(iq.z))))) / 4294967295.0;
}
fn skyColor(ndc: vec2f) -> vec3f {
  let a = u.inv * vec4f(ndc, 0.0, 1.0);
  let b = u.inv * vec4f(ndc, 1.0, 1.0);
  let o = a.xyz / a.w;
  let d = normalize(b.xyz / b.w - o);
  let q = floor(d * 900.0);
  let s = step(1.0 - 0.0018 * u.misc.z, hash3(q)) * hash3(q + vec3f(1.0, 2.0, 3.0));
  var c = u.space.rgb + vec3f(s);
  let C = vec3f(0.0, -u.misc.y, 0.0);
  let oc = C - o;
  let t = dot(oc, d);
  if (t > 0.0) {
    let dm = length(oc - d * t);
    let g = exp(-max(dm - u.misc.y, 0.0) / (u.misc.y * 0.035)) * smoothstep(u.misc.y * 0.96, u.misc.y, dm);
    c = mix(c, u.glow.rgb, clamp(g, 0.0, 1.0));
  }
  let sky = mix(u.horizon.rgb, u.zenith.rgb, smoothstep(0.0, 0.45, d.y));
  return mix(c, sky, u.misc.x);
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  return vec4f(themed(skyColor(i.ndc), 0.0), 1.0);
}
`;var Df=class extends D{kind=`sky`;prog=null;ubuf=null;u=/* @__PURE__ */ new Float32Array(36);constructor(e={}){super({id:`sky`,...e})}async attach(e){await super.attach(e);let t=e.graphics;t&&(this.ubuf=t.buffer(`uniform`,144,`sky`),this.prog=t.program({label:`sky`,wgsl:Ef,glsl:Tf,blocks:[`Frame`,`U`],depth:{write:!1,test:`always`}}),await this.prog.ready)}frame(e){if(!this.prog||!this.ubuf)return[];let t=this.props,n=this.u;return n.set(l.inverse(e.camera.viewProj),0),n.set([...t.space??[.015,.02,.04],1],16),n.set([...t.glow??[.55,.75,1],1],20),n.set([...t.horizon??[.82,.88,.94],1],24),n.set([...t.zenith??[.45,.62,.86],1],28),n.set([e.camera.morph,J,t.stars??1,0],32),this.ubuf.write(n),[{program:this.prog,blocks:[e.frame,this.ubuf],count:3}]}detach(){this.prog?.destroy(),this.ubuf?.destroy(),this.prog=null,this.ubuf=null,super.detach()}};const Of=(e,t)=>`
struct Sea { a: vec4f, b: vec4f, shallow: vec4f, deep: vec4f, field: vec4f, fieldN: vec4f, shore: vec4f, reach: vec4f, centre: vec4f, more: vec4f, w: array<vec4f, 16>, p: array<vec4f, 16> }
@group(0) @binding(${e}) var<uniform> sea: Sea;
@group(1) @binding(${t}) var seaField: texture_2d<f32>;
struct Shore { foam: f32, height: f32, slope: f32, sheet: f32, wet: f32 }
fn seaHash(p: vec2f) -> f32 {
  var q = fract(p * vec2f(0.1031, 0.103));
  q = q + dot(q, q.yx + 33.33);
  return fract((q.x + q.y) * q.x);
}
fn seaNoise(p: vec2f) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(seaHash(i), seaHash(i + vec2f(1.0, 0.0)), w.x), mix(seaHash(i + vec2f(0.0, 1.0)), seaHash(i + vec2f(1.0, 1.0)), w.x), w.y);
}
// a place in the waves' frame (fixed to the ground: the waves stay where they are as the eye moves)
fn seaQ(world: vec3f) -> vec2f {
  return world.xz * sea.b.y;
}
fn seaCell(i: i32, j: i32) -> f32 {
  let n = vec2i(i32(sea.fieldN.x), i32(sea.fieldN.y));
  return textureLoad(seaField, clamp(vec2i(i, j), vec2i(0, 0), n - vec2i(1, 1)), 0).x;
}
// the coast at a place: its signed distance (m: + at sea, − on land), the way away from the land (x, z), and 1 where
// the field knows it (in its window, within its band of the coast), 0 where the ground's height decides
fn seaCoast(world: vec3f) -> vec4f {
  if (sea.field.w < 0.5) {
    return vec4f(1000000.0, 0.0, 0.0, 0.0);
  }
  let c = sea.field.z;
  let x = (world.x - sea.field.x) / c - 0.5;
  let y = (world.z - sea.field.y) / c - 0.5;
  if (x < 0.0 || y < 0.0 || x > sea.fieldN.x - 1.0 || y > sea.fieldN.y - 1.0) {
    return vec4f(1000000.0, 0.0, 0.0, 0.0);
  }
  let i = i32(floor(x));
  let j = i32(floor(y));
  let fx = x - floor(x);
  let fy = y - floor(y);
  let a = seaCell(i, j);
  let b = seaCell(i + 1, j);
  let d = seaCell(i, j + 1);
  let e = seaCell(i + 1, j + 1);
  if (max(max(abs(a), abs(b)), max(abs(d), abs(e))) > sea.fieldN.z) {
    return vec4f(1000000.0, 0.0, 0.0, 0.0);
  }
  let v = mix(mix(a, b, fx), mix(d, e, fx), fy);
  return vec4f(v, mix(b - a, e - d, fy) / c, mix(d - a, e - b, fx) / c, 1.0);
}
// the directional waves at a place of the waves' frame, waves i0 .. i1: (∂h/∂x, ∂h/∂z, h m, the slopes' variance
// left out). A wave shorter than about two footprints (m a pixel) is left out and its roughness kept: the sun's
// glitter widens far away instead of flickering. Near the coast (d m out: inside the surf zone) they die down, the
// shore's waves taking over
fn seaWaves(q: vec2f, i0: i32, i1: i32, foot: f32, d: f32) -> vec4f {
  var o = vec4f(0.0);
  let calm = smoothstep(0.0, max(sea.shore.x, 1.0), d);
  for (var i = i0; i < i1; i = i + 1) {
    let w = sea.w[i];
    let s = w.z * w.w * calm;
    let keep = 1.0 - smoothstep(0.35, 0.7, foot * w.z / 6.283185);
    let ph = w.z * dot(w.xy, q) + sea.p[i].x;
    let c = cos(ph) * s * keep;
    o = o + vec4f(w.x * c, w.y * c, sin(ph) * w.w * calm * keep, 0.5 * s * s * (1.0 - keep * keep));
  }
  return o;
}
// where a point of the sea's own surface goes (Gerstner: the water moves round under the crests, so they sharpen):
// x, up, z in the waves' frame; a wave shorter than 6 to 12 of the surface's steps there (m) left out
fn seaDisplace(q: vec2f, n: i32, spacing: f32, d: f32) -> vec3f {
  var o = vec3f(0.0);
  let calm = smoothstep(0.0, max(sea.shore.x, 1.0), d);
  for (var i = 0; i < n; i = i + 1) {
    let w = sea.w[i];
    let a = w.w * calm * (1.0 - smoothstep(0.08, 0.16, spacing * w.z / 6.283185));
    let ph = w.z * dot(w.xy, q) + sea.p[i].x;
    let k = sea.p[i].y * a * cos(ph);
    o = o + vec3f(w.x * k, a * sin(ph), w.y * k);
  }
  return o;
}
// the shore d m from the coast (+ at sea): the waves come in from the surf zone's edge in order, slowing as the water
// shallows (√(g h), the depth by the shelf), each its own size (a set: some bigger), breaking where it is as deep as it
// is tall into a face of foam with a trail of it behind, then running up the sand as a thin sheet and back, the sand
// wet behind it. q: where along the shore (each place's waves come a little earlier or later)
fn seaShore(d: f32, q: vec2f) -> Shore {
  var o = Shore(0.0, 0.0, 0.0, 0.0, 0.0);
  let surf = max(sea.shore.x, 1.0);
  let runup = sea.shore.y;
  let T = sea.shore.z;
  if (T <= 0.0 || d > surf * 1.3 || d < -runup * 1.6 - 2.0) {
    return o;
  }
  let shelf = max(sea.deep.w, 0.002);
  let along = (seaNoise(q / 150.0) - 0.5) * 0.5 * T + (seaNoise(q / 45.0) - 0.5) * 0.3 * T;
  let tau = 2.0 * sqrt(max(d, 0.0) / (9.81 * shelf));
  let u = (sea.b.z + tau + along) / T;
  let n = floor(u);
  let f = u - n;
  let big = 0.5 + 0.5 * seaHash(vec2f(n, 7.0)) * (0.55 + 0.45 * seaNoise(q / 37.0 + vec2f(n * 1.7, 0.0)));
  let brk = surf * (0.5 + 0.5 * big);
  let back = exp(-f / 0.16);
  let face = exp(-(1.0 - f) / 0.035);
  let crest = max(back, face);
  // grown as it shoals to where it breaks, then the broken wave (a bore of white water) lower to the coast
  var env = clamp(d / brk, 0.15, 1.0);
  if (d >= brk) {
    env = sqrt(clamp(1.0 - (d - brk) / (surf * 1.3 - brk + 1.0), 0.0, 1.0));
  }
  o.height = big * 0.9 * crest * env * step(0.0, d);
  let dfdd = 1.0 / (T * sqrt(9.81 * shelf * max(d, 0.5)));
  o.slope = big * 0.9 * env * step(0.0, d) * select(-back / 0.16, face / 0.035, face > back) * dfdd;
  let broken = smoothstep(brk, brk * 0.82, d) * step(0.0, d);
  // its foam streaked along the shore (across it: the distance itself; along it, a fixed slant of the ground's frame:
  // the coast's own direction jumps from cell to cell, its noise would show the seams), moving with the wave
  let s = vec2f(q.x * 0.8 + q.y * 0.6, d);
  let blot = 0.45 + 0.55 * seaNoise(vec2f(s.x / 5.0, s.y / 1.3) + vec2f(n * 3.7, f * 2.0)) * seaNoise(vec2f(s.x / 1.7, s.y / 0.9) + vec2f(0.0, n));
  o.foam = broken * (exp(-f / 0.12) + exp(-f / 0.45) * 0.7 + face) * blot + broken * 0.22 * smoothstep(0.5, 0.85, seaNoise(vec2f(s.x / 2.3, s.y / 1.1) + vec2f(n, f)));
  // the swash: since the wave reached the coast (f0 of a period), its water up the sand and back
  let u0 = (sea.b.z + along) / T;
  let f0 = u0 - floor(u0);
  let big0 = 0.55 + 0.45 * seaHash(vec2f(floor(u0), 7.0));
  let reach = runup * big0 * max(0.0, 1.0 - pow(2.0 * f0 / 0.75 - 1.0, 2.0)) * step(f0, 0.75);
  let up = step(0.05, reach);
  o.sheet = smoothstep(-reach - 0.6, -reach + 0.4, d) * step(d, 0.5) * up;
  o.wet = smoothstep(-runup * big0 - 1.0, -runup * big0 * 0.5, d) * (0.6 + 0.4 * (1.0 - f0));
  if (d < 0.0) {
    o.foam = exp(-abs(d + reach) / 0.5) * up * select(0.55, 1.0, f0 < 0.375) * blot;
  }
  o.foam = clamp(o.foam * sea.shore.w, 0.0, 1.0);
  return o;
}
// the sky seen in the water in a direction: the haze at the horizon, the sky's light above it (as the frame carries
// them: a sky's own colours, or a plain day's)
fn seaSky(r: vec3f) -> vec3f {
  let up = clamp(r.y, 0.0, 1.0);
  var hz = vec3f(0.62, 0.72, 0.82) * (0.25 + 0.75 * frame.sun.w);
  if (frame.sunColor.w > 0.5) {
    hz = frame.haze.rgb;
  }
  let zen = select(vec3f(0.3, 0.45, 0.7) * (0.2 + 0.8 * frame.sun.w), frame.ambient.rgb * 1.5, frame.sunColor.w > 0.5);
  return mix(hz, zen, smoothstep(0.0, 0.45, pow(up, 0.7)));
}
// foam's colour: white, lit as the ground is (the sky's light and the sun's; a plain day's without a sky)
fn seaFoamLit() -> vec3f {
  let fl = select(vec3f(0.55 + 0.45 * frame.sun.w), frame.ambient.rgb * 1.7 + frame.sunColor.rgb * (0.3 + 0.7 * max(frame.sun.y, 0.0)) * 0.8, frame.sunColor.w > 0.5);
  return vec3f(0.92, 0.95, 0.97) * fl;
}
// the sea's colour at a place: its water (shallow to deep by its depth m), the sky in it and the sun's glitter by its
// normal (a lobe as wide as the slopes left out make it: Cox and Munk), the foam on it
fn seaColour(world: vec3f, n: vec3f, depth: f32, rough: f32, foam: f32) -> vec3f {
  let v = normalize(frame.eye.xyz - world);
  let nv = max(dot(n, v), 0.0);
  let fres = 0.02 + 0.98 * pow(1.0 - nv, 5.0);
  let r = reflect(-v, n);
  let day = frame.sun.w;
  let lit = frame.ambient.rgb * 0.9 + frame.sunColor.rgb * max(frame.sun.y, 0.0) * 0.55;
  let k = 1.0 - exp(-max(depth, 0.0) / max(sea.shallow.w, 0.1));
  let body = mix(sea.shallow.rgb, sea.deep.rgb, k) * select(vec3f(0.45 + 0.55 * day), lit, frame.sunColor.w > 0.5);
  var c = mix(body, seaSky(r), fres);
  let m2 = 0.0016 + (rough + sea.more.x) * 2.0;
  let h = normalize(v + frame.sun.xyz);
  let nh = max(dot(n, h), 0.05);
  let spec = exp(-(1.0 - nh * nh) / (nh * nh * m2)) / (3.141593 * m2 * nh * nh * nh * nh);
  c = c + frame.sunColor.rgb * min(spec, 60.0) * fres * sea.b.x * 0.08 * step(0.0, frame.sun.y);
  return mix(c, seaFoamLit(), clamp(foam, 0.0, 1.0));
}
// metres a pixel at a place, along the view (a grazing look stretches it: the waves left out there sooner)
fn seaFoot(world: vec3f) -> f32 {
  let e = frame.eye.xyz - world;
  return length(e) * 2.0 * frame.misc.z / max(frame.fbsize.y, 1.0) / max(abs(e.y) / max(length(e), 0.001), 0.003);
}
// the ground's colour c at a place under the sea's look: water where the sea is (the coast deciding near it, else the
// ground's height: below 0..1, the ground's own height m × exaggeration), and on the shore's land side the sand wet and
// darker behind the swash, the swash a thin sheet with the sky in it and the sand through it, its edge foam
fn seaOver(c: vec3f, world: vec3f, height: f32, below: f32) -> vec3f {
  let cs = seaCoast(world);
  let q = seaQ(world);
  var water = below;
  var sh = Shore(0.0, 0.0, 0.0, 0.0, 0.0);
  if (cs.w > 0.5) {
    water = smoothstep(-0.25, 0.25, cs.x);
    sh = seaShore(cs.x, q);
  }
  if (water <= 0.0 && sh.wet <= 0.0 && sh.sheet <= 0.0 && sh.foam <= 0.0) {
    return c;
  }
  let foot = seaFoot(world);
  let wv = seaWaves(q, 0, i32(sea.a.w), foot, cs.x);
  let n = normalize(vec3f(-wv.x * sea.b.y - sh.slope * cs.y, 1.0, -wv.y * sea.b.y - sh.slope * cs.z));
  // (a floor at the level itself: the terrain knows no depth there (a sea flattened to 0): deep)
  var depth = select(sea.b.w - height, 1000.0, sea.b.w - height < 0.05);
  if (cs.w > 0.5) {
    depth = mix(max(cs.x, 0.0) * sea.deep.w, max(depth, 0.0), smoothstep(0.55, 0.95, cs.x / max(sea.fieldN.z, 1.0)));
  }
  let wc = seaColour(world, n, depth, wv.w + sea.more.y * smoothstep(0.02, 1.5, foot), sh.foam);
  let v = normalize(frame.eye.xyz - world);
  var land = c * (1.0 - 0.38 * sh.wet);
  let film = mix(land * 0.75, seaSky(reflect(-v, vec3f(0.0, 1.0, 0.0))), 0.35 + 0.4 * pow(1.0 - max(v.y, 0.0), 3.0));
  land = mix(mix(land, film, sh.sheet * 0.85), seaFoamLit(), sh.foam);
  return mix(land, wc, water);
}
// the ground's height under the sea's look (m × exaggeration): the water at the sea level over the floor (lower near
// the eye while the sea's own surface draws there, so its troughs never show it), the coast deciding near it (the sand
// over the level, the floor under it)
fn seaGround(p: vec3f, h: f32, level: f32) -> f32 {
  let cs = seaCoast(p);
  var water = h < level + 0.3;
  if (cs.w > 0.5) {
    water = cs.x > 0.0;
  }
  if (!water) {
    if (cs.w > 0.5) {
      return max(h, level + 0.05 - 0.02 * cs.x);
    }
    return h;
  }
  let e = distance(p.xz, sea.centre.xy);
  let drop = sea.fieldN.w * sea.a.y * (1.0 - smoothstep(0.7, 0.95, e / max(sea.reach.w, 1.0))) * smoothstep(0.0, 6.0, cs.x);
  return level - drop;
}
`,kf=`${nn}
${Of(1,0)}
${rt(1)}
const NA = 192;
struct O {
  @builtin(position) pos: vec4f,
  @location(0) world: vec3f,
  @location(1) q: vec2f,
  @location(2) edge: f32,
}
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let j = i32(vid) / (NA + 1);
  let i = i32(vid) % (NA + 1);
  let k = sea.reach.y;
  let r = sea.reach.x * (pow(k, f32(j)) - 1.0) / (k - 1.0);
  let a = f32(i) / f32(NA) * 6.283185;
  var p = vec3f(sea.centre.x + r * cos(a), sea.b.w, sea.centre.y + r * sin(a));
  let q = seaQ(p);
  let cs = seaCoast(p);
  let fade = 1.0 - smoothstep(0.6, 0.95, r / max(sea.reach.w, 1.0));
  var lift = seaDisplace(q, i32(sea.a.z), sea.reach.x * pow(k, f32(j)), cs.x) / sea.b.y * fade;
  if (cs.w > 0.5) {
    let sh = seaShore(cs.x, q);
    lift.y = lift.y + sh.height * fade * smoothstep(0.0, 4.0, cs.x);
  }
  p = p + lift;
  var o: O;
  o.pos = clipOut(frame.viewProj * vec4f(p, 1.0));
  o.world = p;
  o.q = q;
  o.edge = smoothstep(0.75, 0.95, r / max(sea.reach.w, 1.0));
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let cs = seaCoast(i.world);
  let foot = seaFoot(i.world);
  let wv = seaWaves(i.q, 0, i32(sea.a.w), foot, cs.x);
  var foam = 0.0;
  var sx = 0.0;
  var sz = 0.0;
  var land = 0.0;
  if (cs.w > 0.5) {
    let sh = seaShore(cs.x, i.q);
    foam = sh.foam * smoothstep(0.0, 1.0, cs.x);
    sx = sh.slope * cs.y;
    sz = sh.slope * cs.z;
    land = 1.0 - smoothstep(-0.2, 0.3, cs.x);
  }
  let n = normalize(vec3f(-wv.x * sea.b.y - sx, 1.0, -wv.y * sea.b.y - sz));
  let depth = select(1000.0, max(cs.x, 0.0) * sea.deep.w, cs.w > 0.5);
  var c = seaColour(i.world, n, depth, wv.w + sea.more.y * smoothstep(0.02, 1.5, foot), foam) * mix(1.0, 0.55 + 0.45 * sunShadow(i.world, vec3f(0.0, 1.0, 0.0)), frame.sunColor.w);
  let d = distance(i.world, frame.eye.xyz);
  let f = smoothstep(sea.centre.z, sea.centre.w, d) * frame.size.w;
  c = airFog(mix(themedGround(c), select(themedGround(c), frame.haze.rgb, frame.haze.w > 0.0), f), i.world);
  let alpha = sea.a.y * (1.0 - i.edge) * (1.0 - land);
  if (frame.pick.x > 0.5) {
    return vec4f(0.0);
  }
  return vec4f(c, alpha);
}
`,Af=(e,t)=>`
layout(std140) uniform Sea { vec4 a; vec4 b; vec4 shallow; vec4 deep; vec4 field; vec4 fieldN; vec4 shore; vec4 reach; vec4 centre; vec4 more; vec4 w[16]; vec4 p[16]; } sea;
uniform highp sampler2D seaField;
struct Shore { float foam; float height; float slope; float sheet; float wet; };
float seaHash(vec2 p) {
  vec2 q = fract(p * vec2(0.1031, 0.103));
  q = q + dot(q, q.yx + 33.33);
  return fract((q.x + q.y) * q.x);
}
float seaNoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 w = f * f * (3.0 - 2.0 * f);
  return mix(mix(seaHash(i), seaHash(i + vec2(1.0, 0.0)), w.x), mix(seaHash(i + vec2(0.0, 1.0)), seaHash(i + vec2(1.0, 1.0)), w.x), w.y);
}
vec2 seaQ(vec3 world) {
  return world.xz * sea.b.y;
}
float seaCell(int i, int j) {
  ivec2 n = ivec2(int(sea.fieldN.x), int(sea.fieldN.y));
  return texelFetch(seaField, clamp(ivec2(i, j), ivec2(0, 0), n - ivec2(1, 1)), 0).x;
}
vec4 seaCoast(vec3 world) {
  if (sea.field.w < 0.5) {
    return vec4(1000000.0, 0.0, 0.0, 0.0);
  }
  float c = sea.field.z;
  float x = (world.x - sea.field.x) / c - 0.5;
  float y = (world.z - sea.field.y) / c - 0.5;
  if (x < 0.0 || y < 0.0 || x > sea.fieldN.x - 1.0 || y > sea.fieldN.y - 1.0) {
    return vec4(1000000.0, 0.0, 0.0, 0.0);
  }
  int i = int(floor(x));
  int j = int(floor(y));
  float fx = x - floor(x);
  float fy = y - floor(y);
  float a = seaCell(i, j);
  float b = seaCell(i + 1, j);
  float d = seaCell(i, j + 1);
  float e = seaCell(i + 1, j + 1);
  if (max(max(abs(a), abs(b)), max(abs(d), abs(e))) > sea.fieldN.z) {
    return vec4(1000000.0, 0.0, 0.0, 0.0);
  }
  float v = mix(mix(a, b, fx), mix(d, e, fx), fy);
  return vec4(v, mix(b - a, e - d, fy) / c, mix(d - a, e - b, fx) / c, 1.0);
}
vec4 seaWaves(vec2 q, int i0, int i1, float foot, float d) {
  vec4 o = vec4(0.0);
  float calm = smoothstep(0.0, max(sea.shore.x, 1.0), d);
  for (int i = i0; i < i1; i = i + 1) {
    vec4 w = sea.w[i];
    float s = w.z * w.w * calm;
    float keep = 1.0 - smoothstep(0.35, 0.7, foot * w.z / 6.283185);
    float ph = w.z * dot(w.xy, q) + sea.p[i].x;
    float c = cos(ph) * s * keep;
    o = o + vec4(w.x * c, w.y * c, sin(ph) * w.w * calm * keep, 0.5 * s * s * (1.0 - keep * keep));
  }
  return o;
}
vec3 seaDisplace(vec2 q, int n, float spacing, float d) {
  vec3 o = vec3(0.0);
  float calm = smoothstep(0.0, max(sea.shore.x, 1.0), d);
  for (int i = 0; i < n; i = i + 1) {
    vec4 w = sea.w[i];
    float a = w.w * calm * (1.0 - smoothstep(0.08, 0.16, spacing * w.z / 6.283185));
    float ph = w.z * dot(w.xy, q) + sea.p[i].x;
    float k = sea.p[i].y * a * cos(ph);
    o = o + vec3(w.x * k, a * sin(ph), w.y * k);
  }
  return o;
}
Shore seaShore(float d, vec2 q) {
  Shore o = Shore(0.0, 0.0, 0.0, 0.0, 0.0);
  float surf = max(sea.shore.x, 1.0);
  float runup = sea.shore.y;
  float T = sea.shore.z;
  if (T <= 0.0 || d > surf * 1.3 || d < -runup * 1.6 - 2.0) {
    return o;
  }
  float shelf = max(sea.deep.w, 0.002);
  float along = (seaNoise(q / 150.0) - 0.5) * 0.5 * T + (seaNoise(q / 45.0) - 0.5) * 0.3 * T;
  float tau = 2.0 * sqrt(max(d, 0.0) / (9.81 * shelf));
  float u = (sea.b.z + tau + along) / T;
  float n = floor(u);
  float f = u - n;
  float big = 0.5 + 0.5 * seaHash(vec2(n, 7.0)) * (0.55 + 0.45 * seaNoise(q / 37.0 + vec2(n * 1.7, 0.0)));
  float brk = surf * (0.5 + 0.5 * big);
  float back = exp(-f / 0.16);
  float face = exp(-(1.0 - f) / 0.035);
  float crest = max(back, face);
  float env = clamp(d / brk, 0.15, 1.0);
  if (d >= brk) {
    env = sqrt(clamp(1.0 - (d - brk) / (surf * 1.3 - brk + 1.0), 0.0, 1.0));
  }
  o.height = big * 0.9 * crest * env * step(0.0, d);
  float dfdd = 1.0 / (T * sqrt(9.81 * shelf * max(d, 0.5)));
  o.slope = big * 0.9 * env * step(0.0, d) * (face > back ? face / 0.035 : -back / 0.16) * dfdd;
  float broken = smoothstep(brk, brk * 0.82, d) * step(0.0, d);
  vec2 s = vec2(q.x * 0.8 + q.y * 0.6, d);
  float blot = 0.45 + 0.55 * seaNoise(vec2(s.x / 5.0, s.y / 1.3) + vec2(n * 3.7, f * 2.0)) * seaNoise(vec2(s.x / 1.7, s.y / 0.9) + vec2(0.0, n));
  o.foam = broken * (exp(-f / 0.12) + exp(-f / 0.45) * 0.7 + face) * blot + broken * 0.22 * smoothstep(0.5, 0.85, seaNoise(vec2(s.x / 2.3, s.y / 1.1) + vec2(n, f)));
  float u0 = (sea.b.z + along) / T;
  float f0 = u0 - floor(u0);
  float big0 = 0.55 + 0.45 * seaHash(vec2(floor(u0), 7.0));
  float reach = runup * big0 * max(0.0, 1.0 - pow(2.0 * f0 / 0.75 - 1.0, 2.0)) * step(f0, 0.75);
  float up = step(0.05, reach);
  o.sheet = smoothstep(-reach - 0.6, -reach + 0.4, d) * step(d, 0.5) * up;
  o.wet = smoothstep(-runup * big0 - 1.0, -runup * big0 * 0.5, d) * (0.6 + 0.4 * (1.0 - f0));
  if (d < 0.0) {
    o.foam = exp(-abs(d + reach) / 0.5) * up * (f0 < 0.375 ? 1.0 : 0.55) * blot;
  }
  o.foam = clamp(o.foam * sea.shore.w, 0.0, 1.0);
  return o;
}
vec3 seaSky(vec3 r) {
  float up = clamp(r.y, 0.0, 1.0);
  vec3 hz = vec3(0.62, 0.72, 0.82) * (0.25 + 0.75 * frame.sun.w);
  if (frame.sunColor.w > 0.5) {
    hz = frame.haze.rgb;
  }
  vec3 zen = frame.sunColor.w > 0.5 ? frame.ambient.rgb * 1.5 : vec3(0.3, 0.45, 0.7) * (0.2 + 0.8 * frame.sun.w);
  return mix(hz, zen, smoothstep(0.0, 0.45, pow(up, 0.7)));
}
vec3 seaFoamLit() {
  vec3 fl = frame.sunColor.w > 0.5 ? frame.ambient.rgb * 1.7 + frame.sunColor.rgb * (0.3 + 0.7 * max(frame.sun.y, 0.0)) * 0.8 : vec3(0.55 + 0.45 * frame.sun.w);
  return vec3(0.92, 0.95, 0.97) * fl;
}
vec3 seaColour(vec3 world, vec3 n, float depth, float rough, float foam) {
  vec3 v = normalize(frame.eye.xyz - world);
  float nv = max(dot(n, v), 0.0);
  float fres = 0.02 + 0.98 * pow(1.0 - nv, 5.0);
  vec3 r = reflect(-v, n);
  float day = frame.sun.w;
  vec3 lit = frame.ambient.rgb * 0.9 + frame.sunColor.rgb * max(frame.sun.y, 0.0) * 0.55;
  float k = 1.0 - exp(-max(depth, 0.0) / max(sea.shallow.w, 0.1));
  vec3 body = mix(sea.shallow.rgb, sea.deep.rgb, k) * (frame.sunColor.w > 0.5 ? lit : vec3(0.45 + 0.55 * day));
  vec3 c = mix(body, seaSky(r), fres);
  float m2 = 0.0016 + (rough + sea.more.x) * 2.0;
  vec3 h = normalize(v + frame.sun.xyz);
  float nh = max(dot(n, h), 0.05);
  float spec = exp(-(1.0 - nh * nh) / (nh * nh * m2)) / (3.141593 * m2 * nh * nh * nh * nh);
  c = c + frame.sunColor.rgb * min(spec, 60.0) * fres * sea.b.x * 0.08 * step(0.0, frame.sun.y);
  return mix(c, seaFoamLit(), clamp(foam, 0.0, 1.0));
}
float seaFoot(vec3 world) {
  vec3 e = frame.eye.xyz - world;
  return length(e) * 2.0 * frame.misc.z / max(frame.fbsize.y, 1.0) / max(abs(e.y) / max(length(e), 0.001), 0.003);
}
vec3 seaOver(vec3 c, vec3 world, float height, float below) {
  vec4 cs = seaCoast(world);
  vec2 q = seaQ(world);
  float water = below;
  Shore sh = Shore(0.0, 0.0, 0.0, 0.0, 0.0);
  if (cs.w > 0.5) {
    water = smoothstep(-0.25, 0.25, cs.x);
    sh = seaShore(cs.x, q);
  }
  if (water <= 0.0 && sh.wet <= 0.0 && sh.sheet <= 0.0 && sh.foam <= 0.0) {
    return c;
  }
  float foot = seaFoot(world);
  vec4 wv = seaWaves(q, 0, int(sea.a.w), foot, cs.x);
  vec3 n = normalize(vec3(-wv.x * sea.b.y - sh.slope * cs.y, 1.0, -wv.y * sea.b.y - sh.slope * cs.z));
  float depth = sea.b.w - height < 0.05 ? 1000.0 : sea.b.w - height;
  if (cs.w > 0.5) {
    depth = mix(max(cs.x, 0.0) * sea.deep.w, max(depth, 0.0), smoothstep(0.55, 0.95, cs.x / max(sea.fieldN.z, 1.0)));
  }
  vec3 wc = seaColour(world, n, depth, wv.w + sea.more.y * smoothstep(0.02, 1.5, foot), sh.foam);
  vec3 v = normalize(frame.eye.xyz - world);
  vec3 land = c * (1.0 - 0.38 * sh.wet);
  vec3 film = mix(land * 0.75, seaSky(reflect(-v, vec3(0.0, 1.0, 0.0))), 0.35 + 0.4 * pow(1.0 - max(v.y, 0.0), 3.0));
  land = mix(mix(land, film, sh.sheet * 0.85), seaFoamLit(), sh.foam);
  return mix(land, wc, water);
}
float seaGround(vec3 p, float h, float level) {
  vec4 cs = seaCoast(p);
  bool water = h < level + 0.3;
  if (cs.w > 0.5) {
    water = cs.x > 0.0;
  }
  if (!water) {
    if (cs.w > 0.5) {
      return max(h, level + 0.05 - 0.02 * cs.x);
    }
    return h;
  }
  float e = distance(p.xz, sea.centre.xy);
  float drop = sea.fieldN.w * sea.a.y * (1.0 - smoothstep(0.7, 0.95, e / max(sea.reach.w, 1.0))) * smoothstep(0.0, 6.0, cs.x);
  return level - drop;
}
`,jf=`${mt}
${Af(1,0)}
const int NA = 192;
`,Mf={vertex:`${jf}
out vec3 v_world;
out vec2 v_q;
out float v_edge;
void main() {
  int vid = gl_VertexID;
  int j = vid / (NA + 1);
  int i = vid % (NA + 1);
  float k = sea.reach.y;
  float r = sea.reach.x * (pow(k, float(j)) - 1.0) / (k - 1.0);
  float a = float(i) / float(NA) * 6.283185;
  vec3 p = vec3(sea.centre.x + r * cos(a), sea.b.w, sea.centre.y + r * sin(a));
  vec2 q = seaQ(p);
  vec4 cs = seaCoast(p);
  float fade = 1.0 - smoothstep(0.6, 0.95, r / max(sea.reach.w, 1.0));
  vec3 lift = seaDisplace(q, int(sea.a.z), sea.reach.x * pow(k, float(j)), cs.x) / sea.b.y * fade;
  if (cs.w > 0.5) {
    Shore sh = seaShore(cs.x, q);
    lift.y = lift.y + sh.height * fade * smoothstep(0.0, 4.0, cs.x);
  }
  p = p + lift;
  gl_Position = clipOut(frame.viewProj * vec4(p, 1.0));
  v_world = p;
  v_q = q;
  v_edge = smoothstep(0.75, 0.95, r / max(sea.reach.w, 1.0));
}
`,fragment:`${jf}
${Tn(1)}
in vec3 v_world;
in vec2 v_q;
in float v_edge;
out vec4 outColor;
void main() {
  vec4 cs = seaCoast(v_world);
  float foot = seaFoot(v_world);
  vec4 wv = seaWaves(v_q, 0, int(sea.a.w), foot, cs.x);
  float foam = 0.0;
  float sx = 0.0;
  float sz = 0.0;
  float land = 0.0;
  if (cs.w > 0.5) {
    Shore sh = seaShore(cs.x, v_q);
    foam = sh.foam * smoothstep(0.0, 1.0, cs.x);
    sx = sh.slope * cs.y;
    sz = sh.slope * cs.z;
    land = 1.0 - smoothstep(-0.2, 0.3, cs.x);
  }
  vec3 n = normalize(vec3(-wv.x * sea.b.y - sx, 1.0, -wv.y * sea.b.y - sz));
  float depth = cs.w > 0.5 ? max(cs.x, 0.0) * sea.deep.w : 1000.0;
  vec3 c = seaColour(v_world, n, depth, wv.w + sea.more.y * smoothstep(0.02, 1.5, foot), foam) * mix(1.0, 0.55 + 0.45 * sunShadow(v_world, vec3(0.0, 1.0, 0.0)), frame.sunColor.w);
  float d = distance(v_world, frame.eye.xyz);
  float f = smoothstep(sea.centre.z, sea.centre.w, d) * frame.size.w;
  c = airFog(mix(themedGround(c), frame.haze.w > 0.0 ? frame.haze.rgb : themedGround(c), f), v_world);
  float alpha = sea.a.y * (1.0 - v_edge) * (1.0 - land);
  if (frame.pick.x > 0.5) {
    outColor = vec4(0.0);
    return;
  }
  outColor = vec4(c, alpha);
}
`},Nf=`${mt}
layout(std140) uniform U { vec4 a; vec4 b; vec4 empty; vec4 horizon; vec4 tint; vec4 grade; vec4 base; vec4 missing; vec4 sea; } u;
layout(std140) uniform Params { /*@params*/ } params;
const int N = 33;
const int M = 35;
const float RE = 6378137.0;
const float PI = 3.141592653589793;
/*@functions*/
`,Pf=Af(3,5),Ff={vertex:`${Nf}
${Pf}
uniform highp sampler2D tiles;
uniform highp sampler2DArray heights;
out vec2 v_uv;
flat out float v_slot;
out vec3 v_world;
out float v_light;
out vec2 v_uv2;
flat out vec2 v_under;
out vec2 v_tuv;
out float v_height;
out vec3 v_normal;
out vec2 v_ouv;
flat out float v_oslot;
flat out float v_known;
float s2(float a) {
  float s = sin(a * 0.5);
  return 2.0 * s * s;
}
vec4 rec(int ii, int k) {
  return texelFetch(tiles, ivec2(k, ii), 0);
}
float hLoad(int slot, int x, int y) {
  return texelFetch(heights, ivec3(x, y, slot), 0).x;
}
float hGrid(int slot, float x, float y) {
  int x0 = int(clamp(floor(x), 0.0, float(N - 1)));
  int y0 = int(clamp(floor(y), 0.0, float(N - 1)));
  int x1 = min(x0 + 1, N - 1);
  int y1 = min(y0 + 1, N - 1);
  float fx = clamp(x - float(x0), 0.0, 1.0);
  float fy = clamp(y - float(y0), 0.0, 1.0);
  return mix(mix(hLoad(slot, x0, y0), hLoad(slot, x1, y0), fx), mix(hLoad(slot, x0, y1), hLoad(slot, x1, y1), fx), fy);
}
float hAt(vec4 r1, vec4 r3, vec4 r5, int i, int j) {
  float par = 0.0;
  if (r3.w >= 0.0) {
    float g = float(N - 1);
    par = hGrid(int(r3.w), (float(i) / g * r5.x + r5.z) * g, (float(j) / g * r5.y + r5.w) * g);
  }
  if (r1.z < 0.0) {
    return par * u.b.x;
  }
  float r = r3.z * r3.z * (3.0 - 2.0 * r3.z);
  return mix(par, hLoad(int(r1.z), i, j), r) * u.b.x;
}
void main() {
  int ii = gl_InstanceID;
  vec4 r0 = rec(ii, 0);
  vec4 r1 = rec(ii, 1);
  vec4 r2 = rec(ii, 2);
  vec4 r3 = rec(ii, 3);
  vec4 r4 = rec(ii, 4);
  vec4 r5 = rec(ii, 5);
  vec4 r6 = rec(ii, 6);
  int v = gl_VertexID;
  int gi = v % M;
  int gj = v / M;
  int i = clamp(gi - 1, 0, N - 1);
  int j = clamp(gj - 1, 0, N - 1);
  bool skirt = gi == 0 || gj == 0 || gi == M - 1 || gj == M - 1;
  float fu = float(i) / float(N - 1);
  float fv = float(j) / float(N - 1);
  float dl = r0.x + fu * r0.y;
  float lat = 2.0 * atan(exp(r0.z - fv * r1.x)) - PI * 0.5;
  if (r1.w == 1.0 && j == 0) {
    lat = mix(PI * 0.5, lat, u.a.w);
  }
  if (r1.w == 2.0 && j == N - 1) {
    lat = mix(-PI * 0.5, lat, u.a.w);
  }
  float edgeM = RE * u.a.y * r0.y;
  float h0 = hAt(r1, r3, r5, i, j);
  float h = h0;
  if (skirt && (r1.z >= 0.0 || r3.w >= 0.0 || u.b.x > 0.0)) {
    h = min(h0 - min(edgeM * 0.015, 2500.0), -50.0);
  }
  if (sea.a.x > 0.5 && u.sea.r >= 0.0 && !skirt && (r1.z >= 0.0 || r3.w >= 0.0)) {
    float c0 = RE * u.a.y;
    h = seaGround(vec3(c0 * dl, 0.0, -c0 * (r0.w - fv * r1.x)), h, u.sea.a);
  }
  float dp = lat - u.a.x;
  float rr = RE + h;
  float cl = cos(lat);
  float k = s2(dp) + u.a.y * cl * s2(dl);
  vec3 G = vec3(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + u.a.z * cl * s2(dl)));
  float c = RE * u.a.y;
  vec3 F = vec3(c * dl, h, -c * (r0.w - fv * r1.x));
  vec3 position = mix(G, F, u.a.w);
  float light = 1.0;
  vec3 normal = vec3(0.0, 1.0, 0.0);
  if (r1.z >= 0.0 && u.b.y > 0.0) {
    float hx = hAt(r1, r3, r5, min(i + 1, N - 1), j) - hAt(r1, r3, r5, max(i - 1, 0), j);
    float hz = hAt(r1, r3, r5, i, min(j + 1, N - 1)) - hAt(r1, r3, r5, i, max(j - 1, 0));
    float sx = 2.0 * c * r0.y / float(N - 1);
    float sz = 2.0 * c * r1.x / float(N - 1);
    normal = normalize(vec3(-hx / sx, 1.0, hz / sz));
    light = mix(1.0, clamp(0.35 + 0.85 * dot(normal, normalize(vec3(-0.6, 0.75, -0.6))), 0.45, 1.25), u.b.y);
  }
  vec2 uv = vec2(fu, fv);
  float value = h0;
  float time = frame.eye.w;
  /*@position*/
  // behind whatever lies on it in the depth order (frame: groundClip)
  gl_Position = clipOut(groundClip(frame.viewProj * vec4(position, 1.0)));
  v_uv = uv * r2.xy + r2.zw;
  v_slot = r1.y;
  v_world = position;
  v_light = light;
  v_uv2 = uv * r4.xy + r4.zw;
  v_under = vec2(r3.x, r3.y);
  v_tuv = uv;
  v_height = value;
  v_normal = normal;
  v_ouv = uv * r6.y + r6.zw;
  v_oslot = r6.x;
  v_known = (r1.z >= 0.0 || r3.w >= 0.0) ? 1.0 : 0.0;
}
`,fragment:`${Nf}
uniform highp sampler2DArray imagery;
uniform highp sampler2DArray overlay;
${Tn(4)}
${Pf}
in vec2 v_uv;
flat in float v_slot;
in vec3 v_world;
in float v_light;
in vec2 v_uv2;
flat in vec2 v_under;
in vec2 v_tuv;
in float v_height;
in vec3 v_normal;
in vec2 v_ouv;
flat in float v_oslot;
flat in float v_known;
out vec4 outColor;
void main() {
  vec3 c = u.missing.rgb;
  if (v_under.x >= 0.0) {
    c = textureLod(imagery, vec3(v_uv2, v_under.x), 0.0).rgb;
  }
  if (v_slot >= 0.0) {
    float f = v_under.y;
    c = mix(c, textureLod(imagery, vec3(v_uv, v_slot), 0.0).rgb, f * f * (3.0 - 2.0 * f));
  }
  float lumI = dot(c, vec3(0.299, 0.587, 0.114));
  vec3 flatC = u.base.rgb * mix(1.0, clamp(1.0 + (lumI - 0.22) * 3.2, 0.42, 1.9), u.grade.w);
  c = mix(flatC, c, u.base.a);
  c = mix(c * v_light, sunLit(c, normalize(v_normal), v_world), frame.sunColor.w * u.a.w);
  // (a sea with a look draws on a flat world too: its coast's field says where)
  if (u.sea.r >= 0.0 && (u.b.x > 0.0 || sea.a.x > 0.5)) {
    float below = smoothstep(u.sea.a + 0.2 * u.b.x, u.sea.a - 0.6 * u.b.x, v_height) * v_known;
    if (sea.a.x > 0.5) {
      c = seaOver(c, v_world, v_height, smoothstep(u.sea.a + 0.6, u.sea.a + 0.05, v_height) * v_known);
    } else {
      vec3 v = normalize(frame.eye.xyz - v_world);
      float fres = pow(1.0 - max(v.y, 0.0), 4.0);
      float gl = pow(max(dot(normalize(v + frame.sun.xyz), vec3(0.0, 1.0, 0.0)), 0.0), 140.0) * frame.sunColor.w;
      vec3 wc = mix(u.sea.rgb * (0.45 + 0.55 * frame.sun.w), frame.haze.rgb, 0.35 * fres) + frame.sunColor.rgb * gl * 0.7;
      c = mix(c, wc, below);
    }
  }
  float luma = dot(c, vec3(0.299, 0.587, 0.114));
  c = mix(vec3(luma), c, u.grade.x);
  c = (c - vec3(0.5)) * u.grade.z + vec3(0.5);
  c = max(c * u.grade.y, vec3(0.0));
  c = mix(c, u.tint.rgb * (0.25 + 1.5 * luma), u.tint.a);
  float d = distance(v_world, frame.eye.xyz);
  float f = smoothstep(u.b.z, u.b.w, d) * u.a.w;
  vec4 color = vec4(airFog(mix(themedGround(c), frame.haze.w > 0.0 ? frame.haze.rgb : themedGround(u.horizon.rgb), f), v_world), 1.0);
  vec3 position = v_world;
  vec3 normal = v_normal;
  vec2 uv = v_tuv;
  float value = v_height;
  float time = frame.eye.w;
  /*@color*/
  if (frame.pick.x > 0.5) {
    outColor = vec4(0.0);
    return;
  }
  if (v_oslot >= 0.0) {
    vec4 m = textureLod(overlay, vec3(v_ouv, v_oslot), 0.0);
    color = vec4(color.rgb * (1.0 - m.a) + m.rgb * mix(1.0, 0.3 + 0.7 * frame.sun.w, frame.sunColor.w), 1.0);
  }
  outColor = vec4(color.rgb, 1.0);
}
`},If=`${nn}
// a: (lat0, cos lat0, sin lat0, morph) · b: (exaggeration, shade, fog near, fog far) · tint: (rgb, amount)
// grade: (saturation, brightness, contrast, detail) · base: (rgb, imagery) · missing: (rgb, 1) · sea: (rgb: its
// colour, r < 0 none; its level × the exaggeration, m)
struct U { a: vec4f, b: vec4f, empty: vec4f, horizon: vec4f, tint: vec4f, grade: vec4f, base: vec4f, missing: vec4f, sea: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct Params { /*@params*/ }
@group(0) @binding(2) var<uniform> params: Params;
@group(1) @binding(0) var imagery: texture_2d_array<f32>;
@group(1) @binding(1) var tiles: texture_2d<f32>;
@group(1) @binding(2) var heights: texture_2d_array<f32>;
// the ground marks drawn into the tiles (premultiplied): a layer per tile, r6 says which (or an ancestor's piece)
@group(1) @binding(3) var overlay: texture_2d_array<f32>;
@group(1) @binding(6) var smp: sampler;
${rt(4)}
${Of(3,5)}
const N = 33;
const M = 35;
const RE = 6378137.0;
const PI = 3.141592653589793;
/*@functions*/
fn s2(a: f32) -> f32 {
  let s = sin(a * 0.5);
  return 2.0 * s * s;
}
fn rec(ii: i32, k: i32) -> vec4f {
  return textureLoad(tiles, vec2i(k, ii), 0);
}
fn hLoad(slot: i32, x: i32, y: i32) -> f32 {
  return textureLoad(heights, vec2i(x, y), slot, 0).x;
}
fn hGrid(slot: i32, x: f32, y: f32) -> f32 {
  let x0 = i32(clamp(floor(x), 0.0, f32(N - 1)));
  let y0 = i32(clamp(floor(y), 0.0, f32(N - 1)));
  let x1 = min(x0 + 1, N - 1);
  let y1 = min(y0 + 1, N - 1);
  let fx = clamp(x - f32(x0), 0.0, 1.0);
  let fy = clamp(y - f32(y0), 0.0, 1.0);
  return mix(mix(hLoad(slot, x0, y0), hLoad(slot, x1, y0), fx), mix(hLoad(slot, x0, y1), hLoad(slot, x1, y1), fx), fy);
}
// r1: (mercator span, imagery slot, height slot, pole edge) · r3: (under, fade, rise, parent height slot) · r5: parent uv
fn hAt(r1: vec4f, r3: vec4f, r5: vec4f, i: i32, j: i32) -> f32 {
  var par = 0.0;
  if (r3.w >= 0.0) {
    let g = f32(N - 1);
    par = hGrid(i32(r3.w), (f32(i) / g * r5.x + r5.z) * g, (f32(j) / g * r5.y + r5.w) * g);
  }
  if (r1.z < 0.0) {
    return par * u.b.x;
  }
  let r = r3.z * r3.z * (3.0 - 2.0 * r3.z);
  return mix(par, hLoad(i32(r1.z), i, j), r) * u.b.x;
}
struct O {
  @builtin(position) pos: vec4f,
  @location(0) uv: vec2f,
  @location(1) @interpolate(flat) slot: f32,
  @location(2) world: vec3f,
  @location(3) light: f32,
  @location(4) uv2: vec2f,
  @location(5) @interpolate(flat) under: vec2f,
  @location(6) tuv: vec2f,
  @location(7) height: f32,
  @location(8) normal: vec3f,
  @location(9) ouv: vec2f,
  @location(10) @interpolate(flat) oslot: f32,
  @location(11) @interpolate(flat) known: f32,
}
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let ii = i32(iid);
  let r0 = rec(ii, 0);
  let r1 = rec(ii, 1);
  let r2 = rec(ii, 2);
  let r3 = rec(ii, 3);
  let r4 = rec(ii, 4);
  let r5 = rec(ii, 5);
  let r6 = rec(ii, 6);
  let v = i32(vid);
  let gi = v % M;
  let gj = v / M;
  let i = clamp(gi - 1, 0, N - 1);
  let j = clamp(gj - 1, 0, N - 1);
  let skirt = gi == 0 || gj == 0 || gi == M - 1 || gj == M - 1;
  let fu = f32(i) / f32(N - 1);
  let fv = f32(j) / f32(N - 1);
  let dl = r0.x + fu * r0.y;
  var lat = 2.0 * atan(exp(r0.z - fv * r1.x)) - PI * 0.5;
  // web-map tiles stop at ±85°: on the globe the first and last rows reach the poles
  if (r1.w == 1.0 && j == 0) {
    lat = mix(PI * 0.5, lat, u.a.w);
  }
  if (r1.w == 2.0 && j == N - 1) {
    lat = mix(-PI * 0.5, lat, u.a.w);
  }
  let edgeM = RE * u.a.y * r0.y;
  let h0 = hAt(r1, r3, r5, i, j);
  var h = h0;
  // the skirt: a ring that hangs down, so tiles of different levels never show a crack. Every tile has one while the
  // world has terrain (a tile still without heights of its own or its parent's meets neighbours that have them)
  if (skirt && (r1.z >= 0.0 || r3.w >= 0.0 || u.b.x > 0.0)) {
    h = min(h0 - min(edgeM * 0.015, 2500.0), -50.0);
  }
  // the sea's look: the water at its level over the floor, the coast deciding near it (modules › … › water › sea)
  if (sea.a.x > 0.5 && u.sea.r >= 0.0 && !skirt && (r1.z >= 0.0 || r3.w >= 0.0)) {
    let c0 = RE * u.a.y;
    h = seaGround(vec3f(c0 * dl, 0.0, -c0 * (r0.w - fv * r1.x)), h, u.sea.a);
  }
  let dp = lat - u.a.x;
  let rr = RE + h;
  let cl = cos(lat);
  let k = s2(dp) + u.a.y * cl * s2(dl);
  let G = vec3f(rr * cl * sin(dl), h - rr * k, -rr * (sin(dp) + u.a.z * cl * s2(dl)));
  let c = RE * u.a.y;
  let F = vec3f(c * dl, h, -c * (r0.w - fv * r1.x));
  var position = mix(G, F, u.a.w);
  var light = 1.0;
  var normal = vec3f(0.0, 1.0, 0.0);
  if (r1.z >= 0.0 && u.b.y > 0.0) {
    let hx = hAt(r1, r3, r5, min(i + 1, N - 1), j) - hAt(r1, r3, r5, max(i - 1, 0), j);
    let hz = hAt(r1, r3, r5, i, min(j + 1, N - 1)) - hAt(r1, r3, r5, i, max(j - 1, 0));
    let sx = 2.0 * c * r0.y / f32(N - 1);
    let sz = 2.0 * c * r1.x / f32(N - 1);
    normal = normalize(vec3f(-hx / sx, 1.0, hz / sz));
    light = mix(1.0, clamp(0.35 + 0.85 * dot(normal, normalize(vec3f(-0.6, 0.75, -0.6))), 0.45, 1.25), u.b.y);
  }
  let uv = vec2f(fu, fv);
  let value = h0;
  let time = frame.eye.w;
  /*@position*/
  var o: O;
  // behind whatever lies on it in the depth order (frame: groundClip)
  o.pos = clipOut(groundClip(frame.viewProj * vec4f(position, 1.0)));
  o.uv = uv * r2.xy + r2.zw;
  o.slot = r1.y;
  o.world = position;
  o.light = light;
  o.uv2 = uv * r4.xy + r4.zw;
  o.under = vec2f(r3.x, r3.y);
  o.tuv = uv;
  o.height = value;
  o.normal = normal;
  o.ouv = uv * r6.y + r6.zw;
  o.oslot = r6.x;
  // its heights known (its own or its parent's): only there can the ground lie under the sea
  o.known = select(0.0, 1.0, r1.z >= 0.0 || r3.w >= 0.0);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  // what was there (the old look or a parent), and the tile fading in over it
  var c = u.missing.rgb;
  if (i.under.x >= 0.0) {
    c = textureSampleLevel(imagery, smp, i.uv2, i32(i.under.x), 0.0).rgb;
  }
  if (i.slot >= 0.0) {
    let f = i.under.y;
    c = mix(c, textureSampleLevel(imagery, smp, i.uv, i32(i.slot), 0.0).rgb, f * f * (3.0 - 2.0 * f));
  }
  // imagery 0 → the base colour; detail keeps the imagery's light and dark in it
  let lumI = dot(c, vec3f(0.299, 0.587, 0.114));
  let flatC = u.base.rgb * mix(1.0, clamp(1.0 + (lumI - 0.22) * 3.2, 0.42, 1.9), u.grade.w);
  c = mix(flatC, c, u.base.a);
  // lit by the world's sun and sky (with shadows) on the flat map; the fixed hillshade otherwise
  c = mix(c * i.light, sunLit(c, normalize(i.normal), i.world), frame.sunColor.w * u.a.w);
  // the sea (a layer's: water({ sea })): the ground under its level is water, the sky in it and the sun's glint
  // (a sea with a look draws on a flat world too: its coast's field says where)
  if (u.sea.r >= 0.0 && (u.b.x > 0.0 || sea.a.x > 0.5)) {
    let below = smoothstep(u.sea.a + 0.2 * u.b.x, u.sea.a - 0.6 * u.b.x, i.height) * i.known;
    if (sea.a.x > 0.5) {
      // its look: the waves in its light, its depth, the sun's glitter, the shore's foam and swash (a floor at the level
      // itself is sea: a terrain that knows no depth there flattens it to 0)
      c = seaOver(c, i.world, i.height, smoothstep(u.sea.a + 0.6, u.sea.a + 0.05, i.height) * i.known);
    } else {
      let v = normalize(frame.eye.xyz - i.world);
      let fres = pow(1.0 - max(v.y, 0.0), 4.0);
      let gl = pow(max(dot(normalize(v + frame.sun.xyz), vec3f(0.0, 1.0, 0.0)), 0.0), 140.0) * frame.sunColor.w;
      let wc = mix(u.sea.rgb * (0.45 + 0.55 * frame.sun.w), frame.haze.rgb, 0.35 * fres) + frame.sunColor.rgb * gl * 0.7;
      c = mix(c, wc, below);
    }
  }
  let luma = dot(c, vec3f(0.299, 0.587, 0.114));
  c = mix(vec3f(luma), c, u.grade.x);
  c = (c - vec3f(0.5)) * u.grade.z + vec3f(0.5);
  c = max(c * u.grade.y, vec3f(0.0));
  c = mix(c, u.tint.rgb * (0.25 + 1.5 * luma), u.tint.a);
  let d = distance(i.world, frame.eye.xyz);
  let f = smoothstep(u.b.z, u.b.w, d) * u.a.w;
  // a theme mutes the ground (the haze it fades into is the sky's, dimmed by the theme already); the marks drawn into
  // its tiles mute themselves (a subject's keep their colour)
  var color = vec4f(airFog(mix(themedGround(c), select(themedGround(u.horizon.rgb), frame.haze.rgb, frame.haze.w > 0.0), f), i.world), 1.0);
  let position = i.world;
  let normal = i.normal;
  let uv = i.tuv;
  let value = i.height;
  let time = frame.eye.w;
  /*@color*/
  if (frame.pick.x > 0.5) {
    return vec4f(0.0);
  }
  // the marks on the ground over it, as drawn into the tile
  if (i.oslot >= 0.0) {
    let m = textureSampleLevel(overlay, smp, i.ouv, i32(i.oslot), 0.0);
    color = vec4f(color.rgb * (1.0 - m.a) + m.rgb * mix(1.0, 0.3 + 0.7 * frame.sun.w, frame.sunColor.w), 1.0);
  }
  return vec4f(color.rgb, 1.0);
}
`;function Lf(e,t,n=0){let r=-(e.bearing??0)*G,i=e.pitch??t.pitch,a=Math.cos(i),o=[e.dist*a*Math.sin(r),n+e.dist*Math.sin(i),e.dist*a*Math.cos(r)],[s,c]=t.planes(e.dist),u=i>1.5?[-Math.sin(r),0,-Math.cos(r)]:[0,1,0],d=l.multiply(l.perspective(t.fov,t.width/Math.max(1,t.height),s,c),l.lookAt(o,[0,n,0],u));return{lon:e.lon,lat:e.lat,morph:t.morphAt(e.dist),eye:o,vp:d,H:t.height,fov:t.fov,dist:e.dist,groundY:n}}const Rf=/* @__PURE__ */ new Float64Array(57),zf=/* @__PURE__ */ new Float64Array(57),Bf=(e,t,n)=>e+32*(t+2**e*n);function Vf(e,t,n,r,i){let a=t.lat*G,o=r*G,s=(n-t.lon)*G,c=o-a,l=J+i,u=t.morph,d=Math.sin(c/2),f=Math.sin(s/2),p=2*d*d,m=2*f*f,h=Math.cos(o),g=p+Math.cos(a)*h*m,_=l*h*Math.sin(s),v=i-l*g,y=-l*(Math.sin(c)+Math.sin(a)*h*m),b=J*Math.cos(a),x=Math.log(Math.tan(Math.PI/4+Math.max(-85.0511,Math.min(85.0511,r))*G/2)),S=Math.log(Math.tan(Math.PI/4+Math.max(-85.0511,Math.min(85.0511,t.lat))*G/2)),C=b*s,w=-b*(x-S);Rf[e]=_+(C-_)*u,Rf[e+1]=v+(i-v)*u,Rf[e+2]=y+(w-y)*u}const Hf=e=>Math.log(Math.tan(Math.PI/4+Math.max(-85.0511,Math.min(85.0511,e))*G/2));function Uf(e,t,n,r,i,a,o){Rf[e]=t*(i-n)*G,Rf[e+1]=o,Rf[e+2]=-t*(a-r)}function Wf(e,t){let n={lon:e.lon,lat:e.lat,morph:e.morph},r=e.eye,i=e.vp,a=r[0],o=r[1],s=r[2],c=e.H/(2*Math.tan(e.fov/2)),l=x(),u=e.dist*40+1e3,d=[],f=t.split?.prev,p=t.split?.next,m=e.morph>=1,h=J*Math.cos(e.lat*G),g=Hf(e.lat);p?.clear();let _=(r,v,y)=>{if(d.length>=t.maxTiles)return;let b=2**r,x=v/b*360-180,S=(v+1)/b*360-180,C=Math.PI*(1-2*y/b),w=Math.PI*(1-2*(y+1)/b),T=(2*Math.atan(Math.exp(C))-Math.PI/2)/G,E=(2*Math.atan(Math.exp(w))-Math.PI/2)/G;if(t.bounds&&r>0&&(S<=t.bounds[0]||x>=t.bounds[2]||T<=t.bounds[1]||E>=t.bounds[3]))return;let D=e.lon+Y(x-e.lon),O=Math.max(e.groundY,t.hiOf?.(r,v,y)??0),k=0;for(let t=0;t<3;t++){let r=T+(E-T)*t*.5,i=m?Hf(r):0;for(let t=0;t<3;t++){let a=D+(S-x)*t*.5;m?(Uf(k*3,h,e.lon,g,a,i,0),k++,O>1&&(Uf(k*3,h,e.lon,g,a,i,O),k++)):(Vf(k*3,n,a,r,0),k++,O>1&&(Vf(k*3,n,a,r,O),k++))}}if(e.morph<.98&&r>1){let t=!0;for(let n=0;n<k&&t;n++){let r=n*3;(Rf[r]-l[0])*(a-Rf[r])+(Rf[r+1]-l[1])*(o-Rf[r+1])+(Rf[r+2]-l[2])*(s-Rf[r+2])>=-6378137e3*(1-e.morph)&&(t=!1)}if(t)return}let A=!0,j=!0,M=!0,N=!0,P=!0,F=1/0;for(let e=0;e<k;e++){let t=e*3,n=Rf[t],r=Rf[t+1],c=Rf[t+2],l=i[0]*n+i[4]*r+i[8]*c+i[12],u=i[1]*n+i[5]*r+i[9]*c+i[13],d=i[3]*n+i[7]*r+i[11]*c+i[15];zf[t]=l,zf[t+1]=u,zf[t+2]=d,l<-d||(A=!1),l>d||(j=!1),u<-d||(M=!1),u>d||(N=!1),d<0||(P=!1);let f=Math.hypot(n-a,r-o,c-s);f<F&&(F=f)}if(r>1&&(A||j||M||N||P))return;Vf(k*3,n,Math.min(D+(S-x),Math.max(D,e.lon)),Math.min(T,Math.max(E,e.lat)),O);{let e=k*3,t=Math.hypot(Rf[e]-a,Rf[e+1]-o,Rf[e+2]-s);t<F&&(F=t)}if(r>1&&F>u)return;if((2*Math.PI*6378137*Math.cos((T+E)/2*G)/b/Math.max(F,1)*c>(f?.has(Bf(r,v,y))?t.tilePx*.8:t.tilePx)*(t.far?1+t.far*Math.max(0,F/Math.max(1,e.dist)-1):1)||r<2)&&r<t.maxZoom){p?.add(Bf(r,v,y)),_(r+1,2*v,2*y),_(r+1,2*v+1,2*y),_(r+1,2*v,2*y+1),_(r+1,2*v+1,2*y+1);return}let I=9;for(let e=0;e<k;e++){let t=e*3;zf[t+2]>0&&(I=Math.min(I,Math.max(Math.abs(zf[t]/zf[t+2]),Math.abs(zf[t+1]/zf[t+2]))))}d.push({z:r,x:v,y,d:F,c:I})};return _(0,0,0),Gf(d,t.maxTiles),d}function Gf(e,t){let n=/* @__PURE__ */ new Map;for(let t of e)n.set(Bf(t.z,t.x,t.y),t);let r=(e,t,r)=>{for(let i=e;i>=0;i--){let a=n.get(Bf(i,t>>e-i,r>>e-i));if(a)return a}return null};for(let e=0;e<8;e++){let e=!1;for(let i of[...n.values()]){if(!n.has(Bf(i.z,i.x,i.y)))continue;let a=2**i.z;for(let o=0;o<4;o++){let s=o===0?1:o===1?-1:0,c=o===2?1:o===3?-1:0,l=i.y+c;if(l<0||l>=a)continue;let u=r(i.z,(i.x+s+a)%a,l);if(u&&u.z<i.z-1&&n.size<t-3){n.delete(Bf(u.z,u.x,u.y));for(let e=0;e<4;e++){let t={z:u.z+1,x:2*u.x+(e&1),y:2*u.y+(e>>1),d:u.d,c:u.c};n.set(Bf(t.z,t.x,t.y),t)}e=!0}}}if(!e)break}e.length=0;for(let t of n.values())e.push(t)}function Kf(e,t){if(e!==void 0){if(!Array.isArray(e)||e.length!==4||e.some(e=>typeof e!=`number`||!Number.isFinite(e))||e[0]>=e[2]||e[1]>=e[3])throw new K(`E-DATA-001`,`'where' must be a box [west, south, east, north] in degrees (got ${JSON.stringify(e)})`,{where:t,fix:`write [107.4, -7.1, 107.85, -6.75]`});return e}}const qf=(e,t)=>!e.where||pe(e.where,le(t.z,t.x,t.y)),Jf={normal:`source-over`,multiply:`multiply`,screen:`screen`,add:`lighter`};function Yf(e,t,n){let r=n?.length?n[(t.x+t.y)%n.length]:``;return e.replace(`{z}`,String(t.z)).replace(`{x}`,String(t.x)).replace(`{y}`,String(t.y)).replace(`{-y}`,String(2**t.z-1-t.y)).replace(`{s}`,r)}function Xf(e,t=Di){if(typeof e.url!=`string`||!/\{z\}/.test(e.url)||!/\{x\}/.test(e.url)||!/\{-?y\}/.test(e.url))throw new K(`E-DATA-001`,`an imagery url needs {z}, {x} and {y} (got ${JSON.stringify(e.url)})`,{where:e.id??`imagery`,fix:`write 'https://…/{z}/{x}/{y}.png'`});let n=e.minZoom??0,r=e.maxZoom??19,i=t=>Yf(e.url,t,e.subdomains),a={kind:`imagery`,id:e.id??e.url,attribution:e.attribution??``,terms:Oi(e.terms,t),when:e.when,where:Kf(e.where,e.id??e.url),minZoom:n,maxZoom:r,opacity:e.opacity??1,blend:e.blend??`normal`,version:0,urlFor(e){if(e.z<n)return null;let t=Math.min(e.z,r),a=2**(e.z-t);return i({z:t,x:Math.floor(e.x/a),y:Math.floor(e.y/a)})},opaque:e.opaque??/\.jpe?g(\?|$)/i.test(e.url),async bitmap(e,t){if(e.z<n)return null;let a=Math.min(e.z,r),o=2**(e.z-a),s=Math.floor(e.x/o),c=Math.floor(e.y/o),l=null,u=a,d=s,f=c;for(let e=0;!l&&e<=6&&a-e>=n;e++)u=a-e,d=s>>e,f=c>>e,l=await t.loader.get(i({z:u,x:d,y:f}),{kind:`image`,priority:t.priority,prefetch:t.prefetch,signal:t.signal});if(!l)return null;let p=2**(e.z-u),m=l.width/p;return{im:l,sx:(e.x-d*p)*m,sy:(e.y-f*p)*m,size:m}},async pixels(e,t){return e.z<n||e.z>r?null:t.loader.get(i(e),{kind:`imagePixels`,priority:t.priority,prefetch:t.prefetch,signal:t.signal})},async draw(e,t,n){let r=await a.bitmap(t,n);return r?(await Zf(e,a,t,()=>e.drawImage(r.im,r.sx,r.sy,r.size,r.size,0,0,256,256)),!0):!1}};return a}async function Zf(e,t,n,r){if(e.save(),t.where){let[r,i,a,o]=t.where,s=le(n.z,n.x,n.y);if(!(r<=s[0]&&i<=s[1]&&a>=s[2]&&o>=s[3])){let t=e=>Math.log(Math.tan(Math.PI/4+e*Math.PI/360)),s=2**n.z,c=e=>((e+180)/360*s-n.x)*256,l=e=>((1-t(e)/Math.PI)/2*s-n.y)*256;e.beginPath(),e.rect(c(r),l(o),c(a)-c(r),l(i)-l(o)),e.clip()}}try{return await r()}finally{e.restore()}}const Qf=(e,t,n,r)=>e===`terrarium`?t*256+n+r/256-32768:-1e4+(t*65536+n*256+r)*.1;function $f(e,t,n,r,i,a,o){let s=new Float32Array(o*o),c=(r,i)=>{let a=(i*t+r)*4;return Qf(n,e[a],e[a+1],e[a+2])};for(let e=0;e<o;e++)for(let n=0;n<o;n++){let l=Math.min(t-1,Math.max(0,(i+n/(o-1)/r)*t-.5)),u=Math.min(t-1,Math.max(0,(a+e/(o-1)/r)*t-.5)),d=Math.floor(l),f=Math.floor(u),p=Math.min(t-1,d+1),m=Math.min(t-1,f+1),h=l-d,g=u-f;s[e*o+n]=(c(d,f)*(1-h)+c(p,f)*h)*(1-g)+(c(d,m)*(1-h)+c(p,m)*h)*g}return s}function ep(e,t=Di){let n=e.encoding??`terrarium`,r=e.maxZoom??15,i=Math.max(0,Math.round(e.coarser??2)),a=e=>Math.min(Math.max(0,e-i),r),o=/* @__PURE__ */ new Map,s=async(e,t)=>{if(o.has(e))return o.get(e);let n=await t.loader.get(e,{kind:`pixels`,priority:t.priority,prefetch:t.prefetch,signal:t.signal}),r=n?{w:n.width,d:n.data}:null;return o.set(e,r),o.size>48&&o.delete(o.keys().next().value),r},c=t=>Yf(e.url,t);return{kind:`terrain`,id:e.id??e.url,attribution:e.attribution??``,terms:Oi(e.terms,t),when:e.when,where:Kf(e.where,e.id??e.url),maxZoom:r,blend:e.blend??0,version:0,urlFor(e){let t=a(e.z),n=2**(e.z-t);return c({z:t,x:Math.floor(e.x/n),y:Math.floor(e.y/n)})},async heights(e,t,r){let i=a(e.z),o=2**(e.z-i),l=Math.floor(e.x/o),u=Math.floor(e.y/o),d=await s(c({z:i,x:l,y:u}),r);return d?$f(d.d,d.w,n,o,(e.x-l*o)/o,(e.y-u*o)/o,t):null}}}function tp(e,t){if(!(e.heights instanceof Float32Array)||e.heights.length<e.cols*e.rows)throw new K(`E-DATA-001`,`the grid has ${e.heights?.length??0} heights for ${e.cols} × ${e.rows}`,{where:t,fix:`heights must be a Float32Array of cols × rows, row 0 north`});let n,r;if(e.local){let t=e.local,i=111320*Math.cos(t.lat0*Math.PI/180),a=110574;r=(e,n)=>[((e-t.lon0)*i-t.west)/t.cell-.5,(t.north-(n-t.lat0)*a)/t.cell-.5],n=[t.lon0+t.west/i,t.lat0+(t.north-e.rows*t.cell)/a,t.lon0+(t.west+e.cols*t.cell)/i,t.lat0+t.north/a]}else if(e.bbox){let i=Array.isArray(e.bbox)?e.bbox:[e.bbox.west,e.bbox.south,e.bbox.east,e.bbox.north];n=Kf(i,t),r=(t,n)=>[(t-i[0])/(i[2]-i[0])*e.cols-.5,(i[3]-n)/(i[3]-i[1])*e.rows-.5]}else throw new K(`E-DATA-001`,`a grid needs a bbox or a local frame`,{where:t,fix:`give bbox: [west, south, east, north]`});let i=e.heights,a=e.cols,o=e.nodata;return{box:n,at(t,n){let[s,c]=r(t,n);if(s<-.5||c<-.5||s>a-.5||c>e.rows-.5)return NaN;let l=Math.min(a-1,Math.max(0,s)),u=Math.min(e.rows-1,Math.max(0,c)),d=Math.floor(l),f=Math.floor(u),p=Math.min(a-1,d+1),m=Math.min(e.rows-1,f+1),h=l-d,g=u-f,_=[i[f*a+d],i[f*a+p],i[m*a+d],i[m*a+p]];return o!==void 0&&_.some(e=>e===o)?NaN:(_[0]*(1-h)+_[1]*h)*(1-g)+(_[2]*(1-h)+_[3]*h)*g}}}function np(e,t,n){let r=new Float32Array(t*t);for(let i=0;i<t;i++)for(let a=0;a<t;a++){let[o,s]=I(e.z,e.x,e.y,a/(t-1),i/(t-1));r[i*t+a]=n(o,s)}return r}async function rp(e,t,n,r){let i=null,a=await Promise.all(e.map(e=>qf(e,t)?e.heights(t,n,r):Promise.resolve(null)));if(e.forEach((e,r)=>{let o=a[r];if(!o)return;i||=new Float32Array(n*n).fill(NaN);let s=i;for(let r=0;r<n;r++)for(let i=0;i<n;i++){let a=r*n+i,c=o[a];if(Number.isNaN(c))continue;let l=1;if(e.where&&e.blend>0){let[a,o]=I(t.z,t.x,t.y,i/(n-1),r/(n-1));l=Math.max(0,Math.min(1,_(a,o,e.where)/e.blend))}s[a]=Number.isNaN(s[a])||l>=1?c:s[a]+(c-s[a])*l}}),i){let e=i;for(let t=0;t<e.length;t++)Number.isNaN(e[t])&&(e[t]=0)}return i}const Q=(e,t,n)=>e+32*(t+2**e*n),ip=e=>{let t=e%32,n=(e-t)/32,r=2**t;return[t,n%r,Math.floor(n/r)]},ap=(e,t,n)=>`${e}/${t}/${n}`;var op=class{cache=/* @__PURE__ */ new Map;hz=0;last=null;finer=/* @__PURE__ */ new Map;countFiner(e,t){for(let n=e.z-1,r=1;n>=0;n--,r++){let i=Q(n,e.x>>r,e.y>>r),a=(this.finer.get(i)??0)+t;a>0?this.finer.set(i,a):this.finer.delete(i)}}stats={partial:0,holes:0,standIns:0,visible:0,loading:0,cached:0,zMin:0,zMax:0,loads:0,failed:0,missed:0,free:0,waiting:0,inflight:0,aborted:0};recs;drawn;count=0;heightsVersion=0;mid=/* @__PURE__ */ new Set;wanted=/* @__PURE__ */ new Set;near=/* @__PURE__ */ new Set;busyNow=/* @__PURE__ */ new Set;ancestorShown(e){for(let t=1;t<=e.z;t++){let n=this.cache.get(Q(e.z-t,e.x>>t,e.y>>t));if(n&&n.slot>=0)return!0}return!1}wantedOrNear(e){return e.z<=this.o.baseZ||!!e.soon||this.near.has(e.key)}free=[];pending=/* @__PURE__ */ new Set;inflight=0;requestLimit=0;clock=0;now=0;o;alive=!0;constructor(e){this.o={baseZ:2,fade:280,fadeH:600,hold:250,inflight:8,...e},this.recs=new Float32Array(e.maxTiles*28),this.drawn=new Int32Array(e.maxTiles*3);for(let t=e.layers-1;t>=0;t--)this.free.push(t)}get layers(){return this.o.layers}grow(e){for(let t=e-1;t>=this.o.layers;t--)this.free.push(t);this.o.layers=Math.max(this.o.layers,e)}update(e,t,n,r,i=[],a=this.o.inflight){this.clock++,this.now=n,this.requestLimit=a;let o=this.o.baseZ;for(let n of e){let e=n.z-Math.min(n.z,o);this.request(this.entry(n.z-e,n.x>>e,n.y>>e),t)}let s=[...e].sort((e,t)=>e.c-t.c||e.d-t.d),c=this.mid;c.clear();let l=a>this.o.inflight,u=()=>{for(let e of s)if(e.z>o+2){let n=this.cache.get(Q(e.z,e.x,e.y));if(l&&((n?.slot??-1)>=0||n?.pend?.ver===t))continue;let r=Q(e.z-2,e.x>>2,e.y>>2);c.has(r)||(c.add(r),this.request(this.entry(e.z-2,e.x>>2,e.y>>2),t))}};l||u();let d=0;for(let e of s){let r=this.entry(e.z,e.x,e.y);r.slot>=0&&r.ver===t||(r.missing!==t&&!(r.fails>=3&&n<r.retry)&&d++,this.request(r,t))}l&&u(),this.aheadOf(i,t);let f=this.wanted;f.clear();let p=this.near;p.clear();for(let t of e)for(let e=0;e<=3&&e<=t.z;e++)p.add(Q(t.z-e,t.x>>e,t.y>>e));if(this.inflight>0)for(let e of this.busyNow)!e.stale&&!e.soon&&e.z>o&&!p.has(e.key)&&(e.stale=!0,this.inflight--,e.stop?.abort());let m=0,h=1/0,g=!0;for(let n of e){let e=this.cache.get(Q(n.z,n.x,n.y));e&&(f.add(e),e.used=this.clock),(!e||!(e.slot>=0&&e.ver===t||e.pend?.ver===t||e.missing===t))&&(g=!1),e?.pend&&(m++,h=Math.min(h,e.pend.done))}let _=m>0&&(g||n-h>this.o.hold||this.stats.holes>0);if(this.pending.size)for(let e of[...this.pending])(_||!f.has(e))&&this.reveal(e,n);for(let e of this.cache.values())e.prev>=0&&n-e.at>this.o.fade&&(this.release(e.prev),e.prev=-1);this.records(e,t,r),Object.assign(this.stats,{loading:d,cached:this.cache.size,free:this.free.length,waiting:m,inflight:this.inflight})}records(e,t,n){let r=this.recs,i=n.lon,a=q(n.lat),o=this.now,s=this.o,c=0,l=99,u=0,d=0,f=0,p=0,m=(e,t)=>t<=0?1:Math.min(1,Math.max(0,(o-e)/t)),h=(e,t,n,i)=>{let a=2**i;r[e]=1/a,r[e+1]=1/a,r[e+2]=(t-(t>>i<<i))/a,r[e+3]=(n-(n>>i<<i))/a},g=(e,t,n,o,g)=>{let _=_e(e,t,n),v=c*28;r[v]=Y(_.w-i)*G,r[v+1]=(_.e-_.w)*G,r[v+2]=_.n,r[v+3]=_.n-a;let y=!!o&&o.slot>=0;y&&(o.used=this.clock),y?g>0&&f++:d++,y&&o.ver.endsWith(`~partial`)&&p++,r[v+4]=_.n-_.s,r[v+5]=y?o.slot:-1,r[v+6]=y&&g===0&&o.heights?o.slot:-1,r[v+7]=n===0?1:n===2**e-1?2:0,h(v+8,t,n,g);let b=y?m(o.at,s.fade):1,x=-1,S=g;if(b<1&&o.prev>=0)x=o.prev;else if(b<1)for(let r=g+1;r<=e&&x<0;r++){let i=this.cache.get(Q(e-r,t>>r,n>>r));i&&i!==o&&i.slot>=0&&(x=i.slot,S=r,i.used=this.clock)}r[v+12]=x,r[v+13]=b,r[v+14]=y?m(o.hAt,o.hDur??s.fadeH):1,h(v+16,t,n,S);let C=-1;for(let r=1;r<=e&&C<0&&!(e-r<=s.baseZ&&r>5);r++){let i=this.cache.get(Q(e-r,t>>r,n>>r));i&&i.slot>=0&&i.heights&&(C=i.slot,h(v+20,t,n,r),i.used=this.clock)}r[v+15]=C,r[v+24]=-1,r[v+25]=1,r[v+26]=0,r[v+27]=0,this.drawn[3*c]=e,this.drawn[3*c+1]=t,this.drawn[3*c+2]=n,c++,l=Math.min(l,e),u=Math.max(u,e)},_=e=>!!e&&e.slot>=0&&e.ver===t,v=e=>!!e&&e.slot>=0,y=(e,t)=>{let n=this.cache.get(Q(e.z,e.x,e.y));if(t(n))return g(e.z,e.x,e.y,n,0),!0;let r=e.z+1,i=2*e.x,a=2*e.y,o=this.cache.get(Q(r,i,a)),l=this.cache.get(Q(r,i+1,a)),u=this.cache.get(Q(r,i,a+1)),d=this.cache.get(Q(r,i+1,a+1));if(c+4<=s.maxTiles&&t(o)&&t(l)&&t(u)&&t(d))return g(r,i,a,o,0),g(r,i+1,a,l,0),g(r,i,a+1,u,0),g(r,i+1,a+1,d,0),!0;for(let n=1;n<=e.z;n++){let r=this.cache.get(Q(e.z-n,e.x>>n,e.y>>n));if(t(r))return g(e.z,e.x,e.y,r,n),!0}return!1};for(let t of e){if(c>=s.maxTiles)break;!y(t,_)&&!y(t,v)&&g(t.z,t.x,t.y,void 0,0)}this.count=c,Object.assign(this.stats,{visible:c,zMin:c?l:0,zMax:u,holes:d,standIns:f,partial:p})}settle(e,t){let n=0,r=[],i=[],a=this.now,o=this.o;for(let s of e){let e=this.cache.get(Q(s.z,s.x,s.y));e?.missing===t?i.push(ap(s.z,s.x,s.y)):e&&e.fails>=3&&!(e.slot>=0&&e.ver===t)?r.push({tile:ap(s.z,s.x,s.y),error:e.lastError??`failed`}):(!e||e.busy||e.pend||e.hBusy||e.slot<0||e.ver!==t||a-e.at<o.fade||e.heights&&a-e.hAt<(e.hDur??o.fadeH)||e.prev>=0)&&n++}return{done:e.length>0&&n===0,pending:n,failed:r,missing:i}}heightAt(e,t,n=20){let r=q(Math.max(-85.0511,Math.min(85.0511,t))),i=(e+180)/360,a=(Math.PI-r)/(2*Math.PI),o=this.last,s=this.hz>this.o.baseZ+5?this.o.baseZ+1:0;if(o?.h&&o.z<=n&&o.z>=s){let e=2**o.z;if(Math.floor(i*e)===o.x&&Math.floor(a*e)===o.y){let t=o,r=o.z,s=o.key;for(;r<n&&this.finer.has(s);){r++,e*=2,s=Q(r,Math.floor(i*e),Math.floor(a*e));let n=this.cache.get(s);n?.h&&(t=n)}t!==o&&(this.last=t);let c=2**t.z;return this.sample(t.h,i*c-t.x,a*c-t.y)}}for(let e=Math.min(n,this.hz);e>=s;e--){let t=2**e,n=i*t,r=a*t,o=Math.floor(n),s=Math.floor(r),c=this.cache.get(Q(e,o,s));if(c?.h)return this.last=c,this.sample(c.h,n-o,r-s)}return NaN}sample(e,t,n){let r=t*32,i=n*32,a=Math.min(31,Math.floor(r)),o=Math.min(31,Math.floor(i)),s=r-a,c=i-o;return(e[o*33+a]*(1-s)+e[o*33+a+1]*s)*(1-c)+(e[(o+1)*33+a]*(1-s)+e[(o+1)*33+a+1]*s)*c}keptN=0;hiOf(e,t,n){this.hiSeen!==this.keptN&&(this.hiMemo.clear(),this.hiSeen=this.keptN);let r=Q(e,t,n),i=this.hiMemo.get(r);if(i!==void 0)return i;let a=this.hmaxAll;for(let r=0;r<=e;r++){let i=this.cache.get(Q(e-r,t>>r,n>>r))?.hmax;if(i!==void 0){a=i;break}}return this.hiMemo.size>2e4&&this.hiMemo.clear(),this.hiMemo.set(r,a),a}hiMemo=/* @__PURE__ */ new Map;hiSeen=-1;hmaxAll=0;rangeOfTile(e,t,n,r){let i=1/0,a=-1/0,o=0;for(let r=Math.min(e,this.hz),s=e-r;r>=0&&o<4;r--,s++){let e=this.cache.get(Q(r,t>>s,n>>s));e?.h?(o++,e.hmin<i&&(i=e.hmin),e.hmax>a&&(a=e.hmax)):o&&o++}o||(i=0,a=0),r[0]=Math.min(i,0),r[1]=Math.max(a,0)}entry(e,t,n){let r=Q(e,t,n),i=this.cache.get(r);return i||(i={key:r,z:e,x:t,y:n,slot:-1,ver:``,at:0,heights:!1,hAt:0,prev:-1,pend:null,used:this.clock,busy:!1,retry:0,fails:0},this.cache.set(r,i)),i}release(e){e>=0&&this.free.push(e)}slot(e){if(this.free.length)return this.free.pop();let t=null;for(let n of this.cache.values())n!==e&&!n.busy&&n.z>this.o.baseZ&&n.used<this.clock-1&&(n.slot>=0||n.prev>=0||n.pend)&&(!t||n.used<t.used)&&(t=n);return t?(this.release(t.slot),this.release(t.prev),this.release(t.pend?.slot??-1),this.pending.delete(t),this.cache.delete(t.key),this.last===t&&(this.last=null),t.h&&(this.hiSeen=-1,this.countFiner(t,-1)),t.slot=-1,t.prev=-1,t.pend=null,this.free.pop()??-1):-1}reveal(e,t){let n=e.pend;if(!n)return;this.release(e.prev);let r=e.slot>=0||this.ancestorShown(e);e.prev=e.slot,e.slot=n.slot,e.ver=n.ver,e.at=r?t:t-this.o.fade,n.heights&&!e.heights&&(e.hAt=t,e.hDur=r?this.o.fade:0),n.heights&&this.heightsVersion++,e.heights=n.heights,e.pend=null,this.pending.delete(e)}aheadOf(e,t){if(this.inflight>=4)return;let n=this.o.baseZ;for(let r of e){let e=[...r.tiles].sort((e,t)=>e.c-t.c);for(let i=r.depth??1;i>=1;i--)for(let a of e)a.z>n+i&&this.request(this.entry(a.z-i,a.x>>i,a.y>>i),t,`soon`,r.prefetch(a.z-i));for(let n of e)this.request(this.entry(n.z,n.x,n.y),t,`soon`,r.prefetch(n.z));if(this.inflight>=6)break}}had(e,t){let n=0,r=`${t}~ahead`;for(let i of e){let e=this.cache.get(Q(i.z,i.x,i.y));e&&(e.slot>=0&&(e.ver===t||e.ver===r)||e.pend?.ver===t||e.pend?.ver===r||e.missing===t||e.fails>=3)&&n++}return n}request(e,t,n=`now`,r=!1){if(!this.alive||e.busy||this.inflight>=this.requestLimit||this.now<e.retry||e.slot>=0&&e.ver===t||e.pend?.ver===t||e.missing===t||r&&(e.slot>=0&&e.ver===`${t}~ahead`||e.pend?.ver===`${t}~ahead`)||e.partial===t&&this.stats.holes>0&&n===`now`)return;let i=this.slot(e);if(i<0)return;e.busy=!0,this.inflight++,this.stats.loads++,e.soon=n===`soon`,e.stale=!1,this.busyNow.add(e);let a=e.partial!==t,o=new AbortController;e.stop=o,this.o.load(e,t,i,{priority:n,prefetch:r,lenient:e.fails>=3,hurry:a,signal:o.signal}).then(n=>{if(!this.alive)return;if(!n){this.release(i);return}e.pend&&this.release(e.pend.slot);let r=n.partial?`${t}~partial`:n.ahead?`${t}~ahead`:t;e.partial=n.partial?t:void 0,e.pend={slot:i,ver:r,heights:!!n.heights,done:this.now},this.pending.add(e),e.fails=0,e.missing=void 0,n.heights&&this.keepHeights(e,n.heights),n.later&&(e.hBusy=!0,n.later.then(t=>{this.alive&&(e.hBusy=!1,t&&(this.cache.get(e.key)!==e||e.slot!==i&&e.pend?.slot!==i||(this.o.writeHeights?.(i,t),this.keepHeights(e,t),e.pend?.slot===i?e.pend.heights=!0:e.slot===i&&(e.heights=!0,e.hAt=this.now,e.hDur=this.o.fadeH),this.heightsVersion++)))},n=>{this.alive&&(e.hBusy=!1,!o.signal.aborted&&(e.fails++,e.lastError=String(n?.message??n),e.retry=this.now+Math.min(8e3,400*2**e.fails),this.stats.failed++,e.ver===t&&(e.ver=`${t}~noheights`)))}))},n=>{this.alive&&(this.release(i),o.signal.aborted?this.stats.aborted++:this.o.isMiss(n)?(e.missing=t,e.retry=this.now+5e3,this.stats.missed++):(e.fails++,e.lastError=String(n?.message??n),e.retry=this.now+Math.min(8e3,400*2**e.fails),this.stats.failed++))}).finally(()=>{e.busy=!1,this.busyNow.delete(e),e.stale||this.inflight--,e.stale=!1,e.stop===o&&(e.stop=void 0)})}keepHeights(e,t){this.keptN++,!e.h&&this.cache.get(e.key)===e&&this.countFiner(e,1),e.h=t;let n=0,r=1/0;for(let e=0;e<t.length;e++){let i=t[e];i>n&&(n=i),i<r&&(r=i)}e.hmax=n,e.hmin=Number.isFinite(r)?r:0,n>this.hmaxAll&&(this.hmaxAll=n),e.z>this.hz&&(this.hz=e.z)}dispose(){this.alive=!1,this.cache.clear(),this.finer.clear(),this.pending.clear(),this.free=[]}};function sp(e){let t=typeof navigator<`u`?navigator.deviceMemory:void 0;return t?e===`imagery`?t<=2?64:1/0:t<=2?32:t<=4?64:1/0:1/0}const cp=[0,.6,1.5,3,6],lp={imagery:1,base:[.06,.12,.25],saturation:1,brightness:1,contrast:1,tint:[.1,.2,.45],tintAmount:0,detail:.6};var up=class e extends D{kind=`basemap`;tiles=null;g=null;tex=null;recTex=null;hTex=null;recRing=[];recI=0;index=null;ubuf=null;pbuf=null;prog=null;u=/* @__PURE__ */ new Float32Array(36);want=[];ver=``;ahead=[];turns=[];canvas=null;pool=[];s=null;layersN=0;layersCap=0;static imageryStart=32;static imageryStep=16;static overlayStep=8;static growTries=3;growFailed=0;get pools(){return{imagery:this.layersN,imageryCap:this.layersCap,overlays:this.ovlK,overlaysCap:this.ovlCap()}}growImagery(t){let n=this.g;if(!n||!this.tex||!this.hTex||!this.tiles)return;let r=Math.min(this.layersCap,Math.ceil(t/e.imageryStep)*e.imageryStep);if(r<=this.layersN)return;let i=n.texture({label:`surface imagery`,width:256,height:256,layers:r,format:`rgba8`,filter:`linear`,growable:!0}),a=n.texture({label:`surface heights`,width:33,height:33,layers:r,format:`r32f`,growable:!0});!n.copyLayers(this.tex,i,this.layersN)||!n.copyLayers(this.hTex,a,this.layersN)?(i.destroy(),a.destroy(),++this.growFailed>=e.growTries&&(this.layersCap=this.layersN)):(this.growFailed=0,this.tex.destroy(),this.hTex.destroy(),this.tex=i,this.hTex=a,this.tiles.grow(r),this.layersN=r,this.lastDraw?.textures&&(this.lastDraw.textures[0]=i,this.lastDraw.textures[2]=a))}constructor(e){super({id:`surface`,...e})}async attach(t){await super.attach(t);let n=t.graphics;if(!n)return;this.g=n;let r=t.profile;this.layersCap=Math.min(n.caps.maxLayers,r?.tileLayers??160,sp(`imagery`)),this.layersN=n.api===`webgpu`||n.caps.floatTargets?Math.min(this.layersCap,e.imageryStart):this.layersCap,this.tex=n.texture({label:`surface imagery`,width:256,height:256,layers:this.layersN,format:`rgba8`,filter:`linear`,growable:!0}),this.hTex=n.texture({label:`surface heights`,width:33,height:33,layers:this.layersN,format:`r32f`,growable:!0}),this.recRing=[0,1,2].map(e=>n.texture({label:`surface tiles ${e}`,width:7,height:400,format:`rgba32f`})),this.recTex=this.recRing[0],this.ovlTex=n.texture({label:`surface overlay (none)`,width:1,height:1,layers:1,format:`rgba8`,filter:`linear`});let i=[];for(let e=0;e<34;e++)for(let t=0;t<34;t++){let n=e*35+t;i.push(n,n+35,n+1,n+1,n+35,n+35+1)}this.index=n.buffer(`index`,new Uint32Array(i),`surface index`),this.ubuf=n.buffer(`uniform`,144,`surface`),this.paramBlock??=Ye({}),this.pbuf=n.buffer(`uniform`,this.paramBlock.size,`surface params`),this.noSea=n.buffer(`uniform`,/* @__PURE__ */ new Float32Array(168),`surface no sea look`),this.noField=n.texture({label:`surface no coast field`,width:1,height:1,format:`r32f`,filter:`nearest`}),this.prog=this.build(this.props.shader??{}),this.canvas=new OffscreenCanvas(256,256),this.tiles=new op({...t.world.context===`app`?{fade:se*1e3,fadeH:se*1e3}:{},layers:this.layersN,maxTiles:400,inflight:r?.quality===`lite`?10:12,load:(e,t,n,r)=>this.compose(e,t,n,r),writeHeights:(e,t)=>this.hTex?.write(t,{layer:e}),isMiss:e=>e instanceof Ai}),await this.prog.ready}build(e){let t=R({wgsl:If,glsl:Ff},un.surface,e,this.paramBlock??Ye({}),`layer '${this.id}'`);return this.g.program({label:`surface ${this.id}`,wgsl:t.wgsl,glsl:t.glsl,blocks:[`Frame`,`U`,`Params`,`Sea`],textures:[{name:`imagery`,kind:`2d-array`,sample:`float`},{name:`tiles`,kind:`2d`,sample:`unfilterable`},{name:`heights`,kind:`2d-array`,sample:`unfilterable`},{name:`overlay`,kind:`2d-array`,sample:`float`},{name:`shadowMap`,kind:`2d`,sample:`float`},{name:`seaField`,kind:`2d`,sample:`unfilterable`}],depth:{write:!0,test:`less`},cull:`none`})}async setShader(e){if(!this.g){this.props={...this.props,shader:e};return}let t=this.build(e);await t.ready.catch(e=>{throw t.destroy(),e}),this.prog?.destroy(),this.prog=t,this.props={...this.props,shader:e}}prefetch(e,t={}){this.ahead=e.map(e=>({lon:e.lon,lat:e.lat,dist:e.dist??19134411,bearing:e.bearing,pitch:e.pitch??(e.tilt===void 0?void 0:(90-e.tilt)*G)})),this.aheadDepth=t.depth??1,this.aheadWant=null}aheadDepth=1;aheadWant=null;aheadTiles(){let e=this.attached?.camera;return!e||!this.ahead.length?[]:this.aheadWant??=this.ahead.map(t=>Wf(Lf(t,e,e.groundY),this.qopts()))}pumpAhead(){let e=this.tiles;if(!e||!this.ver||!this.ahead.length)return null;let t=this.aheadTiles();e.aheadOf(t.map(e=>({tiles:e,prefetch:()=>!0,depth:this.aheadDepth})),this.ver);let n=0,r=0;for(let i of t)n+=i.length,r+=e.had(i,this.ver);return{needed:n,have:r}}plan(e){let t=this.attached.camera,n=/* @__PURE__ */ new Set;for(let r of e)for(let e of Wf(Lf(r,t),this.qopts())){n.add(`${e.z}/${e.x}/${e.y}`);for(let t of[1,2])e.z-t>2&&n.add(`${e.z-t}/${e.x>>t}/${e.y>>t}`);let t=e.z-Math.min(e.z,2);n.add(`${e.z-t}/${e.x>>t}/${e.y>>t}`)}let r=/* @__PURE__ */ new Set,i=this.s;for(let e of n){let[t,n,a]=e.split(`/`).map(Number),o={z:t,x:n,y:a};for(let e of[...this.props.base?[this.props.base]:[],...this.props.imagery,...this.props.terrain])if((!i||j(e.when,i)>0)&&qf(e,o)){let t=e.urlFor?.(o);t&&r.add(t)}}return[...r]}qopts(){return{tilePx:this.props.tilePx??this.attached?.profile?.tilePx??300,maxZoom:this.props.maxZoom??20,maxTiles:400,hiOf:(e,t,n)=>(this.tiles?.hiOf(e,t,n)??0)*this.exag(),bounds:this.props.bounds}}exag(){return this.s?Pe(this.props.exaggerate??1,this.s):typeof this.props.exaggerate==`number`?this.props.exaggerate:1}active(e){let t=[];for(let n of[...this.props.base?[this.props.base]:[],...this.props.imagery]){let r=Math.round(j(n.when,e,`imagery '${n.id}'`)*4)/4;r>0&&t.push({src:n,k:r})}for(let e of this.attached?.world.elementsOfKind(`raster`)??[]){let n=e.painted?.();n&&t.push({src:n,k:1})}let n=this.props.terrain.filter(t=>j(t.when,e,`terrain '${t.id}'`)>0);return{imagery:t,terrain:n,ver:t.map(e=>`${e.src.id}:${e.src.version}:${e.src.opacity*e.k}:${e.src.blend}`).join(`|`)+`#${n.map(e=>`${e.id}:${e.version}`).join(`|`)}#${(this.props.empty??[.79,.89,.95]).join(`,`)}`}}frame(e){let t=this.g,n=this.tiles;if(!t||!n||!this.prog)return[];this.s=e.s;let r=performance.now(),i=this.attached?.profile?.quality===`lite`?3:6,a=0;for(let e=0;e<i&&this.turns.length&&(e===0||performance.now()-r<2);e++)this.turns.shift()(),a++;let o=performance.now();this.cost.uploadMs=o-r,this.cost.uploads=a,this.cost.queued=this.turns.length;let s=e.camera,c=this.active(e.s);this.ver=c.ver,this.act=c;let l=c.terrain.map(e=>`${e.id}:${e.version}`).join(`|`);l!==this.terrainKey&&(this.terrainKey=l,this.terrainBumps++);let u=this.qopts(),d=this.split;[d.prev,d.next]=[d.next,d.prev],s.moving||d.prev.clear();let f={lon:s.lon,lat:s.lat,morph:s.morph,eye:s.eye,vp:s.viewProj,H:s.height,fov:s.fov,dist:s.dist,groundY:s.groundY},p=Math.floor(this.layersN*.6),m=1/Math.max(.25,this.detailOf(e.s)),h=e=>Wf(f,{...u,tilePx:u.tilePx*m*1.35**Math.max(0,e-cp.length+1),far:cp[Math.min(e,cp.length-1)],maxTiles:Math.min(u.maxTiles,p*2),split:d}),g=s.moving?this.coarse:0;for(this.want=h(g),g===0&&this.want.length>p&&this.layersN<this.layersCap&&(this.growImagery(Math.ceil(this.want.length/.6)),p=Math.floor(this.layersN*.6),this.want=h(0));this.want.length>p&&g<cp.length+3;)this.want=h(++g);if(g>0&&s.moving&&(this.coarseTry=(this.coarseTry+1)%8)==0){let e=h(g-1);e.length<=p&&(this.want=e,g--)}this.coarse=g;let _=this.aheadTiles().map(e=>({tiles:e,prefetch:()=>!0,depth:this.aheadDepth})),v=s.moving?this.attached?.profile?.quality===`lite`?10:12:16;n.update(this.want,c.ver,e.loadNow??e.now,s,_,v),this.cost.tilesMs=performance.now()-o;let y=0;for(let e of this.want)e.z>y&&(y=e.z);this.zMax=y,this.recI=(this.recI+1)%this.recRing.length,this.recTex=this.recRing[this.recI],n.count&&this.recTex.write(n.recs.subarray(0,n.count*28),{width:7,height:n.count});let b=this.props,x={...lp,...b.grade},S=c.terrain.length?this.exag():0,C=s.dist,w=this.u;w.set([s.lat*G,Math.cos(s.lat*G),Math.sin(s.lat*G),s.morph],0),w.set([S,c.terrain.length?b.shade??.6:0,C*6,C*40],4),w.set([...b.empty??[.79,.89,.95],1],8),w.set([...b.horizon??[.82,.88,.94],1],12),w.set([...x.tint,x.tintAmount],16),w.set([x.saturation,x.brightness,x.contrast,x.detail],20),w.set([...x.base,x.imagery],24),w.set([...b.missing??[.05,.08,.12],1],28);let T=e.sea;if(w.set(T?[T.color[0],T.color[1],T.color[2],T.level*S]:[-1,0,0,0],32),this.ubuf.write(w),this.paramsDirty&&this.paramBlock&&(this.pbuf.write(this.paramBlock.pack(this.paramValues)),this.paramsDirty=!1),!n.count)return[];let E=this.prog;return this.lastDraw={program:E,blocks:[e.frame,this.ubuf,this.pbuf,T?.look?.block??this.noSea],textures:[this.tex,this.recTex,this.hTex,this.ovlTex,e.shadowMap,T?.look?.field??this.noField],index:this.index,count:6936,instanceCount:n.count,pickProgram:()=>E},[this.lastDraw]}act=null;lastDraw=null;noSea=null;noField=null;cost={uploadMs:0,uploads:0,tilesMs:0,queued:0};frameCost(){let e=this.cost;return{uploadMs:+e.uploadMs.toFixed(2),uploads:e.uploads,tilesMs:+e.tilesMs.toFixed(2),queued:e.queued}}coarse=0;coarseTry=0;get coarseness(){return this.coarse}split={prev:/* @__PURE__ */ new Set,next:/* @__PURE__ */ new Set};inFrame(e){return new Promise((t,n)=>this.turns.push(()=>{try{t(e())}catch(e){n(e)}}))}canvasOf(){let e=this.pool.pop()??new OffscreenCanvas(256,256),t=e.getContext(`2d`,{willReadFrequently:!0});return t.setTransform(1,0,0,1,0,0),t.globalAlpha=1,t.globalCompositeOperation=`source-over`,t.clearRect(0,0,256,256),e}async compose(e,t,n,r){let i=this.act;if(!i||i.ver!==t)return null;let a={z:e.z,x:e.x,y:e.y},o=this.attached.loader,s=[],c=e=>!r.prefetch||e.terms.prefetch,l={loader:o,priority:r.priority,prefetch:r.prefetch,signal:r.signal},u=Promise.resolve();try{let o=(this.props.upload??`pixels`)===`pixels`?i.imagery.length-1:-1,d=i.imagery.map(()=>void 0),f=null,p=new Promise(e=>{f=e});u=Promise.all(i.imagery.map(async({src:e,k:t},n)=>{try{if(a.z<e.minZoom||!qf(e,a)||!c(e)){d[n]=null;return}try{if(n===o&&e.pixels&&e.opaque&&!e.where&&t*e.opacity>=1&&e.blend===`normal`){let r=await e.pixels(a,l);if(r&&r.width===256&&r.height===256){d[n]={px:r,k:t,src:e};return}}if(e.bitmap&&!e.where){let r=await e.bitmap(a,l);d[n]=r?{b:r,k:t,src:e}:null;return}let r=this.canvasOf();s.push(r),d[n]=await e.draw(r.getContext(`2d`),a,l)?{c:r,k:t,src:e}:null}catch(i){if(r.lenient&&!(i instanceof Ai)&&!(e.opaque&&e.blend===`normal`&&t*e.opacity>=1)){d[n]=null;return}throw i}}finally{d[n]&&f()}}));let m=i.terrain.filter(c),h=m.length?rp(m,a,33,l).catch(e=>{if(r.lenient&&!(e instanceof Ai))return null;throw e}):Promise.resolve(null);h.catch(()=>{});let g=!1;r.hurry&&i.imagery.length>1&&!this.tiles?.ancestorShown(e)?(u.catch(()=>{}),await Promise.race([p,u]),await Promise.race([u,new Promise(e=>setTimeout(e,this.props.patience??150))]),g=d.some(e=>e===void 0)):await u;let _=null,v,y=await Promise.race([h.then(e=>({h:e}),()=>null),new Promise(e=>setTimeout(()=>e(null),0))]);if(y?_=y.h:v=h,t!==this.act?.ver||!this.tiles?.wantedOrNear(e))return null;let b=await this.composed(d).catch(()=>null);if(t!==this.act?.ver||!this.tiles?.wantedOrNear(e))return null;let x=await this.inFrame(()=>this.upload(d,n,t,_,g,v,b)),S=r.prefetch&&(i.imagery.some(({src:e})=>!e.terms.prefetch&&a.z>=e.minZoom&&qf(e,a))||i.terrain.some(e=>!e.terms.prefetch));return x&&S?{...x,ahead:!0}:x}finally{u.catch(()=>{}).finally(()=>this.pool.push(...s))}}shows(e){let t=0;for(let n=e.length-1;n>=0;n--){let r=e[n];if(r&&r.k*r.src.opacity>=1&&r.src.blend===`normal`&&r.src.opaque&&(r.b||r.c||r.px)){t=n;break}}let n=e[t];if(n?.px&&t===e.length-1)return{from:t,whole:!0};let r=e.slice(t).filter(Boolean).length===1&&t===e.length-1||e.slice(t+1).every(e=>!e);return{from:t,whole:!!(n?.b&&r&&t>=0&&n.k*n.src.opacity>=1&&n.src.opaque&&n.b.sx===0&&n.b.sy===0&&n.b.size===256&&n.b.im.width===256&&n.b.im.height===256)}}async composed(e){let{from:t,whole:n}=this.shows(e);if(n)return null;let r=[];for(let n=t;n<e.length;n++){let t=e[n];if(!t)continue;let i=t.k*t.src.opacity,a=Jf[t.src.blend];t.b?r.push({im:t.b.im,sx:t.b.sx,sy:t.b.sy,sw:t.b.size,sh:t.b.size,alpha:i,op:a}):t.c?r.push({im:await createImageBitmap(t.c),sx:0,sy:0,sw:256,sh:256,alpha:i,op:a,own:!0}):t.px&&r.push({im:await createImageBitmap(new ImageData(new Uint8ClampedArray(t.px.data),t.px.width,t.px.height)),sx:0,sy:0,sw:t.px.width,sh:t.px.height,alpha:i,op:a,own:!0})}let[i,a,o]=(this.props.empty??[.79,.89,.95]).map(e=>Math.round(e*255)),s=await mi(256,`rgb(${i},${a},${o})`,r);return s.width===256&&s.height===256?s:null}upload(e,t,n,r,i,a,o=null){{if(n!==this.act?.ver||!this.canvas||!this.tex)return null;if(o)return this.tex.write(o.data,{layer:t}),r&&this.hTex.write(r,{layer:t}),{heights:r,partial:i,later:a};let{from:s,whole:c}=this.shows(e),l=e[s];if(l?.px&&s===e.length-1)return this.tex.write(l.px.data,{layer:t}),r&&this.hTex.write(r,{layer:t}),{heights:r,partial:i,later:a};if(c&&l?.b)this.tex.writeImage(l.b.im,{layer:t});else{let n=this.canvas.getContext(`2d`),[r,i,a]=(this.props.empty??[.79,.89,.95]).map(e=>Math.round(e*255));n.globalAlpha=1,n.globalCompositeOperation=`source-over`,n.fillStyle=`rgb(${r},${i},${a})`,n.fillRect(0,0,256,256);for(let t=s;t<e.length;t++){let r=e[t];r&&(n.globalAlpha=r.k*r.src.opacity,n.globalCompositeOperation=Jf[r.src.blend],r.b?n.drawImage(r.b.im,r.b.sx,r.b.sy,r.b.size,r.b.size,0,0,256,256):n.drawImage(r.c,0,0))}n.globalAlpha=1,n.globalCompositeOperation=`source-over`,this.tex.writeImage(this.canvas,{layer:t})}return r&&this.hTex.write(r,{layer:t}),{heights:r,partial:i,later:a}}}status(){if(!this.attached)return{ready:!1,pending:[`attaching`],failed:[],missing:[]};if(!this.tiles)return super.status();let e=this.tiles.settle(this.want,this.ver),t=e.pending?[`${e.pending} tile(s) loading or fading in`]:this.want.length?[]:[`no frame yet`];return this.ovlPending&&t.push(`${this.ovlPending} tile(s) drawing their ground marks`),{ready:e.done&&!this.ovlPending,pending:t,failed:e.failed.map(e=>({what:`tile ${e.tile}`,error:e.error})),missing:e.missing.map(e=>`tile ${e}`)}}animating(e){return this.ovlPending>0}ovlTex=null;ovlK=0;ovlSize=512;ovlTargets=[];ovlSlots=/* @__PURE__ */ new Map;ovlFree=[];ovlEach=[];ovlBufs=[];ovlT=/* @__PURE__ */ new Float32Array(12);ovlKeys=/* @__PURE__ */ new Set;ovlShift=0;ovlClock=0;ovlPending=0;ovlVer=``;ovlCov=/* @__PURE__ */ new Uint8Array(64);ovlRescale=/* @__PURE__ */ new Set;ovlSig=(/* @__PURE__ */ new Float64Array(11)).fill(NaN);ovlNow=/* @__PURE__ */ new Float64Array(4);ovlSc=/* @__PURE__ */ new Float64Array(15);ovlUsedAt=0;static overlayKeepMs=5e3;static overlayDrift={moving:1.3,still:1.08};ovlCap(){let e=this.g;return e?Math.min(e.caps.maxLayers,this.attached?.profile?.quality===`lite`?64:128,sp(`overlays`)):0}ensureOverlays(t){let n=this.g;if(!n)return;this.ovlUsedAt=performance.now();let r=e.overlayStep,i=this.ovlCap(),a=Math.min(i,Math.max(r,Math.ceil(t/r)*r));if(a<=this.ovlK)return;let o=this.ovlTex,s=this.ovlK;if(this.ovlTex=n.texture({label:`surface overlay`,width:this.ovlSize,height:this.ovlSize,layers:a,format:`rgba8`,filter:`linear`,growable:!0}),s&&o&&!n.copyLayers(o,this.ovlTex,s)){this.ovlSlots.clear(),this.ovlFree=[];for(let e=s-1;e>=0;e--)this.ovlFree.push(e);this.ovlRescale.clear(),this.ovlSig.fill(NaN)}this.lastDraw?.textures&&(this.lastDraw.textures[3]=this.ovlTex);for(let e of this.ovlTargets)e?.destroy();o?.destroy(),this.ovlTargets=Array.from({length:a},()=>null);for(let e=a-1;e>=s;e--)this.ovlFree.unshift(e);this.ovlBufs=[...this.ovlBufs,...Array.from({length:a-s},()=>null)],this.ovlK=a}releaseOverlays(){let e=this.g;if(!e||!this.ovlK)return;for(let e of[...this.ovlTargets,...this.ovlBufs])e?.destroy();let t=this.ovlTex;this.ovlTex=e.texture({label:`surface overlay (none)`,width:1,height:1,layers:1,format:`rgba8`,filter:`linear`}),this.lastDraw?.textures&&(this.lastDraw.textures[3]=this.ovlTex),t?.destroy(),this.ovlTargets=[],this.ovlBufs=[],this.ovlFree=[],this.ovlSlots.clear(),this.ovlK=0,this.ovlVer=``,this.ovlEach=[],this.ovlRescale.clear(),this.ovlSig.fill(NaN)}cornersNow(e,t,n,r){let i=t.eye,a=2*Math.tan(t.fov/2)/Math.max(1,t.height),o=t.lon+Y(e[0]-t.lon),s=o+(e[1]-e[0]);if(t.morph>=1){let c=J*Math.cos(t.lat*G),l=q(Math.max(-85.0511,Math.min(85.0511,t.lat)));for(let u=0;u<4;u++){let d=c*((u&1?s:o)-t.lon)*G-i[0],f=e[6+u]-i[1],p=-c*(e[u&2?5:4]-l)-i[2];n[r+u]=Math.sqrt(d*d+f*f+p*p)*a}}else for(let c=0;c<4;c++){let l=te(t,c&1?s:o,e[c&2?3:2],e[6+c]),u=l[0]-i[0],d=l[1]-i[1],f=l[2]-i[2];n[r+c]=Math.sqrt(u*u+d*d+f*f)*a}}cornerHeights(e){let t=this.act?.terrain.length?this.exag():0;for(let n=0;n<4;n++){let r=t?this.heightAt(e[n&1?1:0],e[n&2?3:2]):0;e[6+n]=Number.isFinite(r)?r*t:0}e[14]=this.groundVersion}ovlBlock(e,t,n,r,i,a){let[o,s,c]=ip(e),l=_e(o,s,c),u=le(o,s,c),d=q(Math.max(-85.0511,Math.min(85.0511,t.lat)));a[0]=l.w,a[1]=l.e,a[2]=u[3],a[3]=u[1],a[4]=l.n,a[5]=l.s,this.cornerHeights(a),this.cornersNow(a,t,a,10);let f=Math.cos((u[1]+u[3])/2*G),p=J*(l.e-l.w)*G/this.ovlSize*(f+(Math.cos(t.lat*G)-f)*Math.min(1,t.morph)),m=Math.max(.25,n),h=a[10]/m/p,g=a[11]/m/p,_=a[12]/m/p,v=a[13]/m/p;return i.set([Y(l.w-t.lon)*G,(l.e-l.w)*G,l.n-d,l.n-l.s,this.ovlSize,r,Math.max(h,g,_,v),p,h,g,_,v]),u}rescaleScan(t,n,r,i){let a=this.ovlSig,o=r.eye,s=r.moving?0:i?2:1,c=this.groundVersion,l=!0,u=t.size;for(let e of t)u=(u*31+e)%2147483647;let d=(e,t)=>{a[e]!==t&&(a[e]=t,l=!1)};if(d(0,o[0]),d(1,o[1]),d(2,o[2]),d(3,r.lon),d(4,r.lat),d(5,r.morph),d(6,r.fov),d(7,r.height),d(8,s),d(9,c),d(10,u),l)return;this.ovlRescale.clear();let f=s===2?1+1e-6:s?e.overlayDrift.still:e.overlayDrift.moving,p=1/f,m=this.ovlNow;for(let e of t){let t=this.ovlSlots.get(e);if(t&&t.ver===n){t.sc[14]!==c&&this.cornerHeights(t.sc),this.cornersNow(t.sc,r,m,0);for(let n=0;n<4;n++){let r=m[n]/t.sc[10+n];if(r>f||r<p){this.ovlRescale.add(e);break}}}}}warmTexture=null;warmTarget=null;warmBlock=null;warmProgress=/* @__PURE__ */ new Map;prepareOverlays(e,t,n){let r=this.g,i=this.tiles;if(!r||!i||!i.count||e.some(e=>!e.ready))return!1;if(!e.length)return!0;this.ovlUsedAt=performance.now(),this.warmTexture??=r.texture({label:`overlay preparation`,width:1,height:1,format:`rgba8`}),this.warmTarget??=r.layerTarget(this.warmTexture,0),this.warmBlock??=r.buffer(`uniform`,48,`overlay preparation`);let a=e.map(e=>e.version.split(`/`).slice(0,3).join(`/`)).join(`|`),o=this.warmProgress.get(a);o||(o={keys:[],i:0},this.warmProgress.set(a,o));for(let e of this.want){let t=Q(e.z,e.x,e.y),n=Math.min(e.z,1),r=Q(e.z-n,e.x>>n,e.y>>n);o.keys.includes(t)||o.keys.push(t),o.keys.includes(r)||o.keys.push(r)}let s=performance.now(),c=[],l=this.ovlT;for(;o.i<o.keys.length;o.i++){if(o.i&&performance.now()-s>2)return r.flush?.(),!1;let i=o.keys[o.i],a=this.ovlBlock(i,t.camera,n,r.api===`webgpu`?1:-1,l,this.ovlSc);this.warmBlock.write(l),c.length=0;for(let t of e)t.tile({key:i,bounds:[l[0],l[1],l[2],l[3]],box:a,block:this.warmBlock},c);r.frame(c,{clear:[0,0,0,0],target:this.warmTarget})}return r.flush?.(),i.settle(this.want,this.ver).done}overlays(t,n,r,i){let a=this.g,o=this.tiles;if(!a||!o)return!1;let s=o.count,c=o.recs,l=o.drawn;if(this.ovlClock++,!t.length)return this.ovlPending=0,this.ovlSlots.size&&(this.ovlSlots.clear(),this.ovlFree=Array.from({length:this.ovlK},(e,t)=>this.ovlK-1-t),this.ovlRescale.clear(),this.ovlSig.fill(NaN)),this.ovlK&&performance.now()-this.ovlUsedAt>e.overlayKeepMs&&this.releaseOverlays(),!0;if(t.some(e=>!e.ready))return this.ovlPending=0,this.ovlUsedAt=performance.now(),!1;let u=t.map(e=>e.where),d=u.some(e=>!e),f=this.ovlCov.length>=s?this.ovlCov:this.ovlCov=new Uint8Array(Math.max(s,64));for(let e=0;e<s;e++){if(d){f[e]=1;continue}let t=le(l[3*e],l[3*e+1],l[3*e+2]),n=(t[2]-t[0])*.1,r=(t[3]-t[1])*.1;f[e]=+!!u.some(e=>e[0]<=t[2]+n&&e[2]>=t[0]-n&&e[1]<=t[3]+r&&e[3]>=t[1]-r)}let p=this.ovlSlots.size===0,m=t.map(e=>e.version).join(`|`),h=m!==this.ovlVer,g=this.ovlVer,_=this.ovlEach;if(this.ovlVer=m,this.ovlEach=t.map(e=>e.version),h&&g&&_.length===t.length&&t.every((e,t)=>e.version===_[t]||e.since?.version===_[t])){let e=[];for(let n of t)if(n.since&&n.version!==n.since.version)for(let t=0;t<n.since.boxes.length;t++)e.push(n.since.boxes[t]);for(let[t,n]of this.ovlSlots){if(n.ver!==g)continue;let[r,i,a]=ip(t),o=le(r,i,a),s=(o[2]-o[0])*.1,c=(o[3]-o[1])*.1,l=!1;for(let t=0;t+3<e.length&&!l;t+=4)l=e[t]<=o[2]+s&&e[t+2]>=o[0]-s&&e[t+1]<=o[3]+c&&e[t+3]>=o[1]-c;l||(n.ver=m)}}let v=n.camera,y=this.ovlKeys,b=/* @__PURE__ */ new Set,x=0;for(;x<=4;x++){y.clear();for(let e=0;e<s;e++){if(!f[e])continue;let t=l[3*e],n=Math.min(x,t);y.add(Q(t-n,l[3*e+1]>>n,l[3*e+2]>>n))}b.clear();for(let e of y){let[t,n,r]=ip(e);b.add(Q(Math.max(0,t-1),t?n>>1:n,t?r>>1:r))}if(x===0&&this.ensureOverlays(y.size+b.size+2),y.size+b.size<=this.ovlK-2)break}this.ovlShift=x;let S=a.api===`webgpu`?1:-1;for(let e of y){let t=this.ovlSlots.get(e);t&&(t.used=this.ovlClock)}let C=/* @__PURE__ */ new Set,w=e=>this.ovlSlots.get(e)?.ver===m;for(let e=0;e<s;e++){if(!f[e])continue;let t=l[3*e],n=l[3*e+1],r=l[3*e+2],i=!1;for(let e=Math.min(x,t);e<=t&&e<=x+4&&!i;e++){let a=Q(t-e,n>>e,r>>e);i=w(a)||h&&e===Math.min(x,t)&&this.ovlSlots.has(a)}if(i)continue;let a=Math.min(t,x+1);C.add(Q(t-a,n>>a,r>>a))}for(let e of C){let t=this.ovlSlots.get(e);t&&(t.used=this.ovlClock)}this.rescaleScan(y,m,v,i);let T=0,E=0,D=i?1/0:p?0:h?this.ovlK:6,O=this.ovlT,k=[],A=e=>{let n=this.ovlSlots.get(e),i=n?n.slot:this.ovlAlloc();if(i<0)return!1;let o=n?.sc??/* @__PURE__ */ new Float64Array(15),s=this.ovlBlock(e,v,r,S,O,o);this.ovlRescale.delete(e);let c=this.ovlBufs[i]??=a.buffer(`uniform`,48,`tile overlay`);c.write(O),k.length=0;for(let n of t)n.tile({key:e,bounds:[O[0],O[1],O[2],O[3]],box:s,block:c},k);let l=this.ovlTargets[i]??=a.layerTarget(this.ovlTex,i);return a.frame(k,{clear:[0,0,0,0],target:l}),n?(n.ver=m,n.used=this.ovlClock):(n={slot:i,ver:m,used:this.ovlClock,sc:o},this.ovlSlots.set(e,n)),!0};for(let e of C)!w(e)&&!A(e)&&E++;for(let e of y)w(e)||(T>=D||!A(e)?E++:T++);for(let e of this.ovlRescale)y.has(e)&&w(e)&&(T>=D||!A(e)?E++:T++);this.ovlPending=E;for(let e=0;e<s;e++){let t=l[3*e],n=l[3*e+1],r=l[3*e+2],i=e*28;if(!f[e]){c[i+24]=-1;continue}let a,o=0;for(let e=Math.min(x,t);!a&&e<=t&&e<=x+4;e++){let i=this.ovlSlots.get(Q(t-e,n>>e,r>>e));i?.ver===m&&(a=i,o=e)}for(let e=Math.min(x,t);!a&&e<=t&&e<=x+4;e++)a=this.ovlSlots.get(Q(t-e,n>>e,r>>e)),o=e;if(!a){c[i+24]=-1;continue}a.used=this.ovlClock;let s=2**o;c[i+24]=a.slot,c[i+25]=1/s,c[i+26]=(n-(n>>o<<o))/s,c[i+27]=(r-(r>>o<<o))/s}return s&&this.recTex.write(c.subarray(0,s*28),{width:7,height:s}),!0}ovlAlloc(){let e=this.ovlFree.pop();if(e!==void 0)return e;let t=null,n=1/0;for(let[e,r]of this.ovlSlots)r.used<this.ovlClock&&r.used<n&&(t=e,n=r.used);if(t===null)return-1;let r=this.ovlSlots.get(t);return this.ovlSlots.delete(t),r.slot}tileStats(){return this.tiles?{...this.tiles.stats,layers:this.layersN,want:this.want.length}:null}rangeIn(e,t,n,r){let i=this.tiles;if(!i)return null;let a=Math.max(n-e,(r-t)*1.5,1e-7),o=Math.max(0,Math.min(16,Math.floor(Math.log2(360/a)))),s=(e,t)=>Math.max(0,Math.min(t-1,Math.floor((e+180)/360*t))),c=(e,t)=>Math.max(0,Math.min(t-1,Math.floor((Math.PI-q(Math.max(-85.0511,Math.min(85.0511,e))))/(2*Math.PI)*t))),l=2**o,u=s(e,l),d=s(n,l),f=c(r,l),p=c(t,l);for(;(d-u+1)*(p-f+1)>9&&o>0;)o--,l=2**o,u=s(e,l),d=s(n,l),f=c(r,l),p=c(t,l);this.memoN!==i.keptN&&(this.memo.clear(),this.memoN=i.keptN);let m=1/0,h=-1/0,g=this.rr;for(let e=u;e<=d;e++)for(let t=f;t<=p;t++){let n=Q(o,e,t),r=this.memo.get(n);r||(i.rangeOfTile(o,e,t,g),r=[g[0],g[1]],this.memo.set(n,r)),r[0]<m&&(m=r[0]),r[1]>h&&(h=r[1])}return[m,h]}rr=[0,0];memo=/* @__PURE__ */ new Map;memoN=-1;heightAt(e,t){return this.act&&!this.act.terrain.length?0:this.tiles?.heightAt(e,t,this.zMax||20)??NaN}get groundVersion(){return(this.tiles?.heightsVersion??0)+this.terrainBumps}terrainBumps=0;terrainKey=``;zMax=0;attribution(){let e=this.act;return e?[...e.imagery.map(e=>e.src.attribution),...e.terrain.map(e=>e.attribution)]:[]}detach(){this.tiles?.dispose(),this.tiles=null;for(let e of[this.tex,this.hTex,...this.recRing,this.index,this.ubuf,this.pbuf,this.prog,this.ovlTex,this.warmTarget,this.warmTexture,this.warmBlock,...this.ovlBufs,...this.ovlTargets,this.noSea,this.noField])e?.destroy();this.noSea=null,this.noField=null,this.warmTarget=this.warmTexture=this.warmBlock=null,this.warmProgress.clear(),this.ovlTex=null,this.ovlBufs=[],this.ovlTargets=[],this.ovlK=0,this.ovlSlots.clear(),this.ovlFree=[],this.recRing=[],this.tex=this.hTex=this.recTex=null,this.index=this.ubuf=this.pbuf=null,this.prog=null,this.turns=[],this.pool=[],this.canvas=null,this.g=null,super.detach()}};const dp={"world-imagery":{path:`World_Imagery`,max:19,attribution:`Imagery © Esri, Maxar, Earthstar Geographics`},topo:{path:`World_Topo_Map`,max:19,attribution:`© Esri, HERE, Garmin, © OpenStreetMap contributors`},streets:{path:`World_Street_Map`,max:19,attribution:`© Esri, HERE, Garmin, © OpenStreetMap contributors`},light:{path:`Canvas/World_Light_Gray_Base`,max:16,attribution:`© Esri, HERE, Garmin, © OpenStreetMap contributors`},dark:{path:`Canvas/World_Dark_Gray_Base`,max:16,attribution:`© Esri, HERE, Garmin, © OpenStreetMap contributors`}},fp={esri(e={}){let t=dp[e.layer??`world-imagery`];if(!t)throw new K(`E-DATA-001`,`unknown Esri layer '${e.layer}'`,{fix:`use ${Object.keys(dp).join(`, `)}`});return Xf({id:`esri-${e.layer??`world-imagery`}`,url:e.url??`https://server.arcgisonline.com/ArcGIS/rest/services/${t.path}/MapServer/tile/{z}/{y}/{x}`,maxZoom:t.max,attribution:t.attribution,...e},Ei)},osm(e={}){return Xf({id:`osm`,url:e.url??`https://tile.openstreetmap.org/{z}/{x}/{y}.png`,maxZoom:19,attribution:`© OpenStreetMap contributors`,...e},e.url?void 0:Ei)},sentinel(e={}){let t=e.year??2020;return Xf({id:`sentinel-${t}`,url:`https://tiles.maps.eox.at/wmts/1.0.0/s2cloudless-${t}_3857/default/g/{z}/{y}/{x}.jpg`,maxZoom:15,attribution:`Sentinel-2 cloudless ${t} by EOX IT Services GmbH (contains modified Copernicus Sentinel data ${t}), CC BY 4.0`,...e})},custom(e){if(`url`in e&&e.url)return Xf({id:e.id??`custom`,...e});let t=e.draw;if(typeof t!=`function`)throw new K(`E-DATA-001`,`imagery.custom needs a url or a draw function`,{where:e.id??`imagery.custom`,fix:`imagery.custom({ url: 'https://…/{z}/{x}/{y}.png' }) or imagery.custom({ draw: (g, tile) => { …; return true } })`});let n={kind:`imagery`,id:e.id??`custom`,attribution:e.attribution??``,terms:{show:!0,prefetch:!0,keep:!0,export:!0,...e.terms},when:e.when,where:Kf(e.where,e.id??`imagery.custom`),minZoom:e.minZoom??0,maxZoom:e.maxZoom??22,opacity:e.opacity??1,blend:e.blend??`normal`,version:0,draw:(e,r,i)=>Zf(e,n,r,()=>t(e,r,i))};return n}},pp={mapzen(e={}){return ep({id:`mapzen`,url:e.url??`https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png`,encoding:`terrarium`,maxZoom:e.maxZoom??15,attribution:`Terrain: Mapzen terrain tiles (AWS Open Data; SRTM, GMTED2010, ETOPO1, GEBCO and others)`,...e})},grid(e){let t=e.id??(`url`in e?`grid:${e.url}`:`grid`),n=null,r=null;if(`data`in e)n=tp(e.data,t);else if(typeof e.url!=`string`)throw new K(`E-DATA-001`,`terrain.grid needs a url or data`,{where:t,fix:`terrain.grid({ url: '/dem/bandung-30m' })`});let i=async i=>n||(r??=(async()=>{let r=e.url.replace(/\.(json|f32)$/,``),o=await i.loader.get(`${r}.json`,{kind:`json`,priority:`now`,prefetch:i.prefetch}),s=await i.loader.get(`${r}.f32`,{kind:`bytes`,priority:`now`,prefetch:i.prefetch});if(!o||!s)throw new K(`E-DATA-001`,`no grid at ${r}.json / .f32`,{where:t,fix:`check the url (without the extension)`});let c=Number(o.cols??o.width),l=Number(o.rows??o.height),u=o.origin;return n=tp({cols:c,rows:l,heights:new Float32Array(s),nodata:o.nodata,...u&&o.west_e!==void 0?{local:{lon0:u.lon,lat0:u.lat,west:Number(o.west_e),north:Number(o.north_n),cell:Number(o.cell)}}:{bbox:o.bbox}},t),a.where||=n.box,n})(),r.catch(()=>{r=null}),r),a={kind:`terrain`,id:t,attribution:e.attribution??``,terms:{show:!0,prefetch:!0,keep:!0,export:!0,...e.terms},when:e.when,where:Kf(e.where,t)??n?.box,maxZoom:22,blend:e.blend??50,version:0,async heights(e,t,n){let r=await i(n);if(!r)return null;let a=np(e,t,r.at);return a.some(e=>!Number.isNaN(e))?a:null},urlFor:`url`in e?()=>`${e.url.replace(/\.(json|f32)$/,``)}.f32`:void 0};return a},custom(e){if(typeof e.heights!=`function`)throw new K(`E-DATA-001`,`terrain.custom needs heights(tile, n)`,{where:e.id??`terrain.custom`,fix:`terrain.custom({ heights: (tile, n) => new Float32Array(n * n) })`});return{kind:`terrain`,id:e.id??`custom-terrain`,attribution:e.attribution??``,terms:{show:!0,prefetch:!0,keep:!0,export:!0,...e.terms},when:e.when,where:Kf(e.where,e.id??`terrain.custom`),maxZoom:e.maxZoom??22,blend:e.blend??0,version:0,async heights(t,n,r){let i=await e.heights(t,n,r);if(i&&i.length!==n*n)throw new K(`E-DATA-001`,`terrain '${e.id??`custom`}' gave ${i.length} heights for ${n} × ${n}`,{where:e.id,fix:`return a Float32Array of n * n`});return i}}}};function mp(e){if(typeof e?.url!=`string`)throw new K(`E-DATA-001`,`earthLite needs the url where you host it`,{fix:`earthLite({ url: '/packs/earth-lite' })`});return Xf({id:`earth-lite`,url:/\{z\}/.test(e.url)?e.url:`${e.url.replace(/\/$/,``)}/{z}/{x}/{y}.${e.format??`jpg`}`,maxZoom:e.maxZoom??5,attribution:e.attribution??`earth-lite basemap`,terms:e.terms})}function hp(e={}){let t=e.imagery??[fp.esri()],n=e.terrain??[pp.mapzen()];for(let[e,r]of[[`imagery`,t],[`terrain`,n]]){if(!Array.isArray(r))throw new K(`E-DATA-001`,`earth({ ${e} }) takes a list`,{fix:e===`terrain`?`terrain: [terrain.mapzen()] (or [] for flat)`:`imagery: [imagery.esri()]`});for(let t of r)if(!t||t.kind!==(e===`imagery`?`imagery`:`terrain`))throw new K(`E-DATA-001`,`earth({ ${e} }) has something that is not ${e===`imagery`?`an imagery`:`a terrain`} source`,{fix:e===`imagery`?`use imagery.esri(), imagery.osm(), imagery.sentinel(), imagery.custom(…)`:`use terrain.mapzen(), terrain.grid(…), terrain.custom(…)`})}let r=e.exaggerate??1;return{kind:`earth`,flat:!1,view:e.view??{lon:110,lat:0,zoom:1.6},exaggeration:e=>n.length?Pe(r,e):1,elements:()=>[...e.sky===!1?[]:[e.sky instanceof D?e.sky:new Df(typeof e.sky==`object`?e.sky:{})],new up({...e.surface,imagery:t,terrain:n,base:e.base??null,exaggerate:r,when:e.when})]}}const gp=(e,t)=>[e/(J*G),Vt(t/J)],_p=(e,t)=>[e*J*G,q(t)*J];function vp(e={}){let t=e.size??1e3,n=e.grid===void 0?10:e.grid;if(!(t>0))throw new K(`E-DATA-001`,`plane size must be > 0 metres (got ${t})`,{fix:`plane({ size: 200 })`});let r=t/2,[i,a]=gp(-r,-r),[o,s]=gp(r,r),c={kind:`imagery`,id:`plane-floor`,attribution:``,terms:{show:!0,prefetch:!0,keep:!0,export:!0},where:[i,a,o,s],minZoom:0,maxZoom:30,opacity:1,blend:`normal`,version:0,async draw(t,i){let a=_e(i.z,i.x,i.y),o=a.w*J*G,s=a.e*J*G,c=a.n*J,l=a.s*J,u=256/(s-o),d=e=>(e-o)*u,f=e=>(c-e)*u;if(t.fillStyle=e.color??`#eeeeee`,t.fillRect(d(Math.max(-r,o)),f(Math.min(r,c)),(Math.min(r,s)-Math.max(-r,o))*u,(Math.min(r,c)-Math.max(-r,l))*u),n){let i=n;for(;i*u<4;)i*=10;t.strokeStyle=e.lines??`#b8b8b8`,t.lineWidth=1,t.beginPath();for(let e=Math.ceil(Math.max(-r,o)/i)*i;e<=Math.min(r,s);e+=i)t.moveTo(d(e),f(Math.min(r,c))),t.lineTo(d(e),f(Math.max(-r,l)));for(let e=Math.ceil(Math.max(-r,l)/i)*i;e<=Math.min(r,c);e+=i)t.moveTo(d(Math.max(-r,o)),f(e)),t.lineTo(d(Math.min(r,s)),f(e));t.stroke()}return!0}};return{kind:`plane`,flat:!0,bounds:[i,a,o,s],view:{lon:0,lat:0,dist:t*1.2,tilt:50,bearing:340},elements:()=>[new Df({horizon:[.86,.88,.9],zenith:[.6,.7,.82]}),new up({imagery:[c],terrain:[],empty:e.beyond??[.82,.84,.86],horizon:[.86,.88,.9],bounds:[i,a,o,s],shade:0})]}}const yp=`${mt}
layout(std140) uniform U { vec4 sun; vec4 moon; vec4 p1; vec4 p2; vec4 grey; vec4 horizon; vec4 space; vec4 glow; vec4 misc; vec4 cloud; vec4 cloudLit; vec4 cloudShade; } u;
`,bp={vertex:`${yp}
out vec2 v_ndc;
void main() {
  int i = gl_VertexID;
  vec2 p = vec2(float((i << 1) & 2), float(i & 2)) * 2.0 - 1.0;
  gl_Position = clipOut(vec4(p, 1.0, 1.0));
  v_ndc = p;
}
`,fragment:`${yp}
in vec2 v_ndc;
out vec4 outColor;
uint pcg(uint v) {
  uint s = v * 747796405u + 2891336453u;
  uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}
float hash3(vec3 q) {
  ivec3 iq = ivec3(q);
  return float(pcg(uint(iq.x) ^ pcg(uint(iq.y) ^ pcg(uint(iq.z))))) / 4294967295.0;
}
vec3 spaceColor(vec3 o, vec3 d) {
  vec3 q = floor(d * 900.0);
  float s = step(0.9982, hash3(q)) * hash3(q + vec3(1.0, 2.0, 3.0));
  vec3 c = u.space.rgb + vec3(s);
  vec3 C = vec3(0.0, -u.misc.x, 0.0);
  vec3 oc = C - o;
  float t = dot(oc, d);
  if (t > 0.0) {
    float dm = length(oc - d * t);
    float g = exp(-max(dm - u.misc.x, 0.0) / (u.misc.x * 0.035)) * smoothstep(u.misc.x * 0.96, u.misc.x, dm);
    c = mix(c, u.glow.rgb, clamp(g, 0.0, 1.0));
  }
  return c;
}
float sunIntensity(float c) {
  return 1000.0 * max(0.0, 1.0 - exp(-((1.6110731556870734 - acos(clamp(c, -1.0, 1.0))) / 1.5)));
}
vec3 totalMie(float t) {
  return 0.434 * (0.2 * t) * 10.0e-18 * vec3(1.8399918514433978e14, 2.7798023919660528e14, 4.0790479543861094e14);
}
float rayleighPhase(float c) {
  return 0.05968310365946075 * (1.0 + c * c);
}
float hgPhase(float c, float g) {
  float g2 = g * g;
  return 0.07957747154594767 * ((1.0 - g2) / pow(1.0 - 2.0 * g * c + g2, 1.5));
}
vec3 rrtOdt(vec3 v) {
  return (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
}
vec3 aces(vec3 c, float exposure) {
  mat3 a = mat3(vec3(0.59719, 0.07600, 0.02840), vec3(0.35458, 0.90834, 0.13383), vec3(0.04823, 0.01566, 0.83777));
  mat3 b = mat3(vec3(1.60475, -0.10208, -0.00327), vec3(-0.53108, 1.10813, -0.07276), vec3(-0.07367, -0.00605, 1.07602));
  return clamp(b * rrtOdt(a * (c * exposure / 0.6)), vec3(0.0), vec3(1.0));
}
vec3 toSRGB(vec3 c) {
  return mix(pow(c, vec3(0.41666)) * 1.055 - vec3(0.055), c * 12.92, step(c, vec3(0.0031308)));
}
vec3 physical(vec3 d) {
  vec3 sd = u.sun.xyz;
  float sunE = sunIntensity(sd.y);
  float sunfade = 1.0 - clamp(1.0 - exp(sd.y * 0.8888889), 0.0, 1.0);
  vec3 betaR = vec3(5.804542996261093e-6, 1.3562911419845635e-5, 3.0265902468824876e-5) * (u.p1.y - (1.0 - sunfade));
  vec3 betaM = totalMie(u.p1.x) * u.p1.z;
  float zen = acos(max(0.0, d.y));
  float inv = 1.0 / (cos(zen) + 0.15 * pow(93.885 - ((zen * 180.0) / 3.141592653589793), -1.253));
  vec3 fex = exp(-(betaR * 8.4e3 * inv + betaM * 1.25e3 * inv));
  float ct = dot(d, sd);
  vec3 bR = betaR * rayleighPhase(ct * 0.5 + 0.5);
  vec3 bM = betaM * hgPhase(ct, u.p1.w);
  vec3 k = (bR + bM) / (betaR + betaM);
  vec3 lin = pow(sunE * k * (1.0 - fex), vec3(1.5));
  lin = lin * mix(vec3(1.0), pow(sunE * k * fex, vec3(0.5)), clamp(pow(1.0 - sd.y, 5.0), 0.0, 1.0));
  vec3 l0 = vec3(0.1) * fex;
  float disc = smoothstep(0.9999566769464484, 0.9999566769464484 + 0.00002, ct);
  l0 = l0 + (sunE * 19000.0 * fex) * disc * u.sun.w;
  vec3 tex = (lin + l0) * 0.04 + vec3(0.0, 0.0003, 0.00075);
  vec3 c = pow(tex, vec3(1.0 / (1.2 + (1.2 * sunfade))));
  c = mix(c, u.grey.rgb, u.p2.x);
  c = toSRGB(aces(c, u.horizon.w));
  vec3 blue = mix(vec3(0.62, 0.78, 0.93), vec3(0.22, 0.47, 0.84), smoothstep(0.0, 0.65, d.y));
  c = mix(c, blue, 0.55 * u.p2.y);
  float halo = pow(max(ct, 0.0), 400.0) * 0.6 + pow(max(ct, 0.0), 24.0) * 0.12;
  c = c + vec3(1.0, 0.92, 0.8) * halo * u.misc.y;
  return mix(c, u.horizon.rgb, 1.0 - smoothstep(0.0, 0.09, d.y));
}
vec4 moonDisc(vec3 d) {
  vec3 m = u.moon.xyz;
  float ct = dot(d, m);
  float r = u.misc.z;
  if (ct < cos(r)) {
    return vec4(0.0);
  }
  vec3 p = (d - m * ct) / sin(r);
  float r2 = dot(p, p);
  if (r2 >= 1.0) {
    return vec4(0.0);
  }
  vec3 n = p - m * sqrt(1.0 - r2);
  float lit = smoothstep(-0.04, 0.1, dot(n, u.sun.xyz));
  float maria = 0.25 * smoothstep(0.6, 0.2, length(p - vec3(0.2, 0.25, 0.0))) + 0.2 * smoothstep(0.5, 0.1, length(p + vec3(0.25, 0.1, 0.0)));
  vec3 surf = vec3(0.95, 0.94, 0.88) * (1.0 - maria);
  float night = 1.0 - smoothstep(-0.1, 0.12, u.sun.y);
  vec3 col = surf * lit + vec3(0.09, 0.1, 0.13) * (1.0 - lit) * night;
  float a = max(lit, night * 0.85) * (1.0 - smoothstep(0.92, 1.0, r2)) * u.moon.w;
  return vec4(col * (1.0 + 0.6 * night), a);
}
float h2(vec2 q) {
  vec2 p = fract(q * vec2(0.1031, 0.103));
  p = p + dot(p, p.yx + 33.33);
  return fract((p.x + p.y) * p.x);
}
float vnoise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 w = f * f * (3.0 - 2.0 * f);
  return mix(mix(h2(i), h2(i + vec2(1.0, 0.0)), w.x), mix(h2(i + vec2(0.0, 1.0)), h2(i + vec2(1.0, 1.0)), w.x), w.y);
}
float fbm(vec2 q) {
  vec2 p = q;
  float s = 0.0;
  float a = 0.5;
  for (int k = 0; k < 5; k = k + 1) {
    s = s + a * vnoise(p);
    p = p * 2.03 + vec2(17.1, 17.1);
    a = a * 0.5;
  }
  return s;
}
float cloudDensity(vec2 xz) {
  float c = 1.0 - u.cloud.x;
  return smoothstep(c * 0.75, c * 0.75 + 0.28, fbm((xz + u.cloud.yz) / 3200.0));
}
vec4 cloudsAt(vec3 o, vec3 d) {
  if (u.cloud.w < 0.0 || u.cloud.x < 0.01 || d.y <= 0.0 || o.y >= u.cloud.w) {
    return vec4(0.0);
  }
  vec3 p = o + d * ((u.cloud.w - o.y) / d.y);
  float dens = cloudDensity(p.xz);
  if (dens < 0.01) {
    return vec4(0.0);
  }
  float towardSun = cloudDensity(p.xz + u.sun.xz * 260.0);
  vec3 col = mix(u.cloudLit.rgb, u.cloudShade.rgb, clamp(towardSun * 1.3 - dens * 0.3, 0.0, 1.0));
  float fs = max(dot(d, u.sun.xyz), 0.0);
  col = col + u.cloudLit.rgb * (pow(fs, 40.0) * (1.4 - dens) * 1.3 + pow(fs, 6.0) * 0.25) * u.sun.w;
  col = col + vec3(0.7, 0.75, 0.9) * u.grey.w;
  float far = length(p.xz - o.xz);
  col = mix(col, u.horizon.rgb, smoothstep(6000.0, 38000.0, far) * 0.85);
  return vec4(col, dens * 0.92 * (1.0 - smoothstep(22000.0, 42000.0, far)) * u.cloudLit.w);
}
void main() {
  vec4 a = frame.inv * vec4(v_ndc, 0.0, 1.0);
  vec4 b = frame.inv * vec4(v_ndc, 1.0, 1.0);
  vec3 o = a.xyz / a.w;
  vec3 d = normalize(b.xyz / b.w - o);
  vec3 c = spaceColor(o, d);
  if (u.p2.w > 0.0) {
    vec3 near = physical(d);
    vec3 q = floor(d * 700.0);
    float s = step(0.9986, hash3(q)) * (0.35 + 0.65 * hash3(q + vec3(1.0, 2.0, 3.0))) * smoothstep(0.02, 0.2, d.y);
    near = near + vec3(0.85, 0.9, 1.0) * s * u.p2.z;
    vec4 mo = moonDisc(d);
    near = mix(near, mo.rgb, mo.a);
    vec4 cl = cloudsAt(o, d);
    near = mix(near, cl.rgb, cl.a);
    near = mix(near, vec3(0.85, 0.88, 1.0), u.grey.w * 0.6);
    c = mix(c, near, u.p2.w);
  }
  outColor = vec4(c, 1.0);
}
`},xp=`${nn}
// sun: (towards it xyz, its disc 0..1) · moon: (towards it xyz, opacity) · p1: (turbidity, rayleigh, mie, mie g)
// p2: (overcast, blue sky lift, stars 0..1, morph) · grey: overcast colour (rgb, flash) · horizon: the haze (rgb, exposure)
// space, glow: the view from afar · misc: (the body's radius, the sun's halo, the moon's angular radius, 0)
// cloud: (cover, where the view's centre is in metres (x, z: drift and pan), the cloud base above sea level, m; < 0
// none) · cloudLit: (their sunlit colour, how much they show from the camera's height) · cloudShade: (their shaded colour)
struct U { sun: vec4f, moon: vec4f, p1: vec4f, p2: vec4f, grey: vec4f, horizon: vec4f, space: vec4f, glow: vec4f, misc: vec4f, cloud: vec4f, cloudLit: vec4f, cloudShade: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct O { @builtin(position) pos: vec4f, @location(0) ndc: vec2f }
@vertex fn vs(@builtin(vertex_index) vid: u32) -> O {
  let i = i32(vid);
  let p = vec2f(f32((i << 1u) & 2), f32(i & 2)) * 2.0 - 1.0;
  var o: O;
  o.pos = clipOut(vec4f(p, 1.0, 1.0));
  o.ndc = p;
  return o;
}
fn pcg(v: u32) -> u32 {
  let s = v * 747796405u + 2891336453u;
  let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}
fn hash3(q: vec3f) -> f32 {
  let iq = vec3i(q);
  return f32(pcg(u32(iq.x) ^ pcg(u32(iq.y) ^ pcg(u32(iq.z))))) / 4294967295.0;
}
// from afar: black space with stars, the glow round the globe's edge
fn spaceColor(o: vec3f, d: vec3f) -> vec3f {
  let q = floor(d * 900.0);
  let s = step(0.9982, hash3(q)) * hash3(q + vec3f(1.0, 2.0, 3.0));
  var c = u.space.rgb + vec3f(s);
  let C = vec3f(0.0, -u.misc.x, 0.0);
  let oc = C - o;
  let t = dot(oc, d);
  if (t > 0.0) {
    let dm = length(oc - d * t);
    let g = exp(-max(dm - u.misc.x, 0.0) / (u.misc.x * 0.035)) * smoothstep(u.misc.x * 0.96, u.misc.x, dm);
    c = mix(c, u.glow.rgb, clamp(g, 0.0, 1.0));
  }
  return c;
}
fn sunIntensity(c: f32) -> f32 {
  return 1000.0 * max(0.0, 1.0 - exp(-((1.6110731556870734 - acos(clamp(c, -1.0, 1.0))) / 1.5)));
}
fn totalMie(t: f32) -> vec3f {
  return 0.434 * (0.2 * t) * 10.0e-18 * vec3f(1.8399918514433978e14, 2.7798023919660528e14, 4.0790479543861094e14);
}
fn rayleighPhase(c: f32) -> f32 {
  return 0.05968310365946075 * (1.0 + c * c);
}
fn hgPhase(c: f32, g: f32) -> f32 {
  let g2 = g * g;
  return 0.07957747154594767 * ((1.0 - g2) / pow(1.0 - 2.0 * g * c + g2, 1.5));
}
fn rrtOdt(v: vec3f) -> vec3f {
  return (v * (v + 0.0245786) - 0.000090537) / (v * (0.983729 * v + 0.4329510) + 0.238081);
}
// filmic tone mapping (ACES, Stephen Hill's fitted curve) and the sRGB curve
fn aces(c: vec3f, exposure: f32) -> vec3f {
  let a = mat3x3f(vec3f(0.59719, 0.07600, 0.02840), vec3f(0.35458, 0.90834, 0.13383), vec3f(0.04823, 0.01566, 0.83777));
  let b = mat3x3f(vec3f(1.60475, -0.10208, -0.00327), vec3f(-0.53108, 1.10813, -0.07276), vec3f(-0.07367, -0.00605, 1.07602));
  return clamp(b * rrtOdt(a * (c * exposure / 0.6)), vec3f(0.0), vec3f(1.0));
}
fn toSRGB(c: vec3f) -> vec3f {
  return mix(pow(c, vec3f(0.41666)) * 1.055 - vec3f(0.055), c * 12.92, step(c, vec3f(0.0031308)));
}
// the daylight sky towards d (Preetham), tone mapped, lifted towards a clear blue, melting into the haze below
fn physical(d: vec3f) -> vec3f {
  let sd = u.sun.xyz;
  let sunE = sunIntensity(sd.y);
  let sunfade = 1.0 - clamp(1.0 - exp(sd.y * 0.8888889), 0.0, 1.0);
  let betaR = vec3f(5.804542996261093e-6, 1.3562911419845635e-5, 3.0265902468824876e-5) * (u.p1.y - (1.0 - sunfade));
  let betaM = totalMie(u.p1.x) * u.p1.z;
  let zen = acos(max(0.0, d.y));
  let inv = 1.0 / (cos(zen) + 0.15 * pow(93.885 - ((zen * 180.0) / 3.141592653589793), -1.253));
  let fex = exp(-(betaR * 8.4e3 * inv + betaM * 1.25e3 * inv));
  let ct = dot(d, sd);
  let bR = betaR * rayleighPhase(ct * 0.5 + 0.5);
  let bM = betaM * hgPhase(ct, u.p1.w);
  let k = (bR + bM) / (betaR + betaM);
  var lin = pow(sunE * k * (1.0 - fex), vec3f(1.5));
  lin = lin * mix(vec3f(1.0), pow(sunE * k * fex, vec3f(0.5)), clamp(pow(1.0 - sd.y, 5.0), 0.0, 1.0));
  var l0 = vec3f(0.1) * fex;
  let disc = smoothstep(0.9999566769464484, 0.9999566769464484 + 0.00002, ct);
  l0 = l0 + (sunE * 19000.0 * fex) * disc * u.sun.w;
  let tex = (lin + l0) * 0.04 + vec3f(0.0, 0.0003, 0.00075);
  var c = pow(tex, vec3f(1.0 / (1.2 + (1.2 * sunfade))));
  c = mix(c, u.grey.rgb, u.p2.x);
  c = toSRGB(aces(c, u.horizon.w));
  let blue = mix(vec3f(0.62, 0.78, 0.93), vec3f(0.22, 0.47, 0.84), smoothstep(0.0, 0.65, d.y));
  c = mix(c, blue, 0.55 * u.p2.y);
  // the sun's halo through haze and thin cloud
  let halo = pow(max(ct, 0.0), 400.0) * 0.6 + pow(max(ct, 0.0), 24.0) * 0.12;
  c = c + vec3f(1.0, 0.92, 0.8) * halo * u.misc.y;
  return mix(c, u.horizon.rgb, 1.0 - smoothstep(0.0, 0.09, d.y));
}
// the moon's disc towards d: (colour, alpha), lit from the sun's direction
fn moonDisc(d: vec3f) -> vec4f {
  let m = u.moon.xyz;
  let ct = dot(d, m);
  let r = u.misc.z;
  if (ct < cos(r)) {
    return vec4f(0.0);
  }
  let p = (d - m * ct) / sin(r);
  let r2 = dot(p, p);
  if (r2 >= 1.0) {
    return vec4f(0.0);
  }
  let n = p - m * sqrt(1.0 - r2);
  let lit = smoothstep(-0.04, 0.1, dot(n, u.sun.xyz));
  let maria = 0.25 * smoothstep(0.6, 0.2, length(p - vec3f(0.2, 0.25, 0.0))) + 0.2 * smoothstep(0.5, 0.1, length(p + vec3f(0.25, 0.1, 0.0)));
  let surf = vec3f(0.95, 0.94, 0.88) * (1.0 - maria);
  let night = 1.0 - smoothstep(-0.1, 0.12, u.sun.y);
  let col = surf * lit + vec3f(0.09, 0.1, 0.13) * (1.0 - lit) * night;
  let a = max(lit, night * 0.85) * (1.0 - smoothstep(0.92, 1.0, r2)) * u.moon.w;
  return vec4f(col * (1.0 + 0.6 * night), a);
}
fn h2(q: vec2f) -> f32 {
  var p = fract(q * vec2f(0.1031, 0.103));
  p = p + dot(p, p.yx + 33.33);
  return fract((p.x + p.y) * p.x);
}
fn vnoise(p: vec2f) -> f32 {
  let i = floor(p);
  let f = fract(p);
  let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(h2(i), h2(i + vec2f(1.0, 0.0)), w.x), mix(h2(i + vec2f(0.0, 1.0)), h2(i + vec2f(1.0, 1.0)), w.x), w.y);
}
fn fbm(q: vec2f) -> f32 {
  var p = q;
  var s = 0.0;
  var a = 0.5;
  for (var k = 0; k < 5; k = k + 1) {
    s = s + a * vnoise(p);
    p = p * 2.03 + vec2f(17.1, 17.1);
    a = a * 0.5;
  }
  return s;
}
fn cloudDensity(xz: vec2f) -> f32 {
  let c = 1.0 - u.cloud.x;
  return smoothstep(c * 0.75, c * 0.75 + 0.28, fbm((xz + u.cloud.yz) / 3200.0));
}
// the cloud layer seen towards d from o (below it): colour and cover, lit from the sun's side
fn cloudsAt(o: vec3f, d: vec3f) -> vec4f {
  if (u.cloud.w < 0.0 || u.cloud.x < 0.01 || d.y <= 0.0 || o.y >= u.cloud.w) {
    return vec4f(0.0);
  }
  let p = o + d * ((u.cloud.w - o.y) / d.y);
  let dens = cloudDensity(p.xz);
  if (dens < 0.01) {
    return vec4f(0.0);
  }
  let towardSun = cloudDensity(p.xz + u.sun.xz * 260.0);
  var col = mix(u.cloudLit.rgb, u.cloudShade.rgb, clamp(towardSun * 1.3 - dens * 0.3, 0.0, 1.0));
  let fs = max(dot(d, u.sun.xyz), 0.0);
  col = col + u.cloudLit.rgb * (pow(fs, 40.0) * (1.4 - dens) * 1.3 + pow(fs, 6.0) * 0.25) * u.sun.w;
  col = col + vec3f(0.7, 0.75, 0.9) * u.grey.w;
  let far = length(p.xz - o.xz);
  col = mix(col, u.horizon.rgb, smoothstep(6000.0, 38000.0, far) * 0.85);
  return vec4f(col, dens * 0.92 * (1.0 - smoothstep(22000.0, 42000.0, far)) * u.cloudLit.w);
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let a = frame.inv * vec4f(i.ndc, 0.0, 1.0);
  let b = frame.inv * vec4f(i.ndc, 1.0, 1.0);
  let o = a.xyz / a.w;
  let d = normalize(b.xyz / b.w - o);
  var c = spaceColor(o, d);
  if (u.p2.w > 0.0) {
    var near = physical(d);
    // stars at night, above the haze
    let q = floor(d * 700.0);
    let s = step(0.9986, hash3(q)) * (0.35 + 0.65 * hash3(q + vec3f(1.0, 2.0, 3.0))) * smoothstep(0.02, 0.2, d.y);
    near = near + vec3f(0.85, 0.9, 1.0) * s * u.p2.z;
    let mo = moonDisc(d);
    near = mix(near, mo.rgb, mo.a);
    let cl = cloudsAt(o, d);
    near = mix(near, cl.rgb, cl.a);
    near = mix(near, vec3f(0.85, 0.88, 1.0), u.grey.w * 0.6);
    c = mix(c, near, u.p2.w);
  }
  return vec4f(c, 1.0);
}
`,Sp=Math.PI/180;function Cp(e,t,n){let r=(typeof e==`number`?e:e.getTime())/864e5+2440587.5-2451545,i=(280.46+.9856474*r)%360,a=(357.528+.9856003*r)%360*Sp,o=(i+1.915*Math.sin(a)+.02*Math.sin(2*a))*Sp,s=(23.439-4e-7*r)*Sp,c=Math.atan2(Math.cos(s)*Math.sin(o),Math.cos(o)),l=Math.asin(Math.sin(s)*Math.sin(o)),u=((18.697374558+24.06570982441908*r)%24*15+n)*Sp-c,d=t*Sp,f=Math.asin(Math.sin(d)*Math.sin(l)+Math.cos(d)*Math.cos(l)*Math.cos(u)),p=Math.atan2(Math.sin(u),Math.cos(u)*Math.sin(d)-Math.tan(l)*Math.cos(d));return{altitude:f/Sp,azimuth:(p/Sp+180+360)%360}}function wp(e,t,n){let r=(typeof e==`number`?e:e.getTime())/864e5-10957.5,i=218.316+13.176396*r,a=(134.963+13.064993*r)*Sp,o=(93.272+13.22935*r)*Sp,s=(297.85+12.190749*r)*Sp,c=(357.529+.98560028*r)*Sp,l=(i+6.289*Math.sin(a)-1.274*Math.sin(a-2*s)+.658*Math.sin(2*s)-.186*Math.sin(c)-.059*Math.sin(2*a-2*s)-.057*Math.sin(a-2*s+c)+.053*Math.sin(a+2*s))*Sp,u=(5.128*Math.sin(o)+.281*Math.sin(a+o)-.278*Math.sin(o-a)-.173*Math.sin(o-2*s))*Sp,d=(23.439-4e-7*r)*Sp,f=Math.atan2(Math.sin(l)*Math.cos(d)-Math.tan(u)*Math.sin(d),Math.cos(l)),p=Math.asin(Math.sin(u)*Math.cos(d)+Math.cos(u)*Math.sin(d)*Math.sin(l)),m=(280.46061837+360.98564736629*r+n)%360*Sp-f,h=t*Sp,g=Math.asin(Math.sin(h)*Math.sin(p)+Math.cos(h)*Math.cos(p)*Math.cos(m));g-=.0166*Math.cos(g);let _=Math.atan2(Math.sin(m),Math.cos(m)*Math.sin(h)-Math.tan(p)*Math.cos(h))+Math.PI,v=(357.528+.9856003*r)*Sp,y=(280.46+.9856474*r+1.915*Math.sin(v)+.02*Math.sin(2*v))%360,b=((l/Sp-y)%360+360)%360,x=(1-Math.cos(Math.acos(Math.cos(u)*Math.cos(b*Sp))))/2;return{altitude:g/Sp,azimuth:(_/Sp+360)%360,illumination:x,elongation:b,phase:b/360}}function Tp(e){let t=e.altitude*Sp,n=e.azimuth*Sp;return[Math.sin(n)*Math.cos(t),Math.sin(t),-Math.cos(n)*Math.cos(t)]}function Ep(e,t,n){let r=null,i=null,a=12,o=-90,s=Cp(e,t,n).altitude;for(let c=5;c<=1440;c+=5){let l=Cp(e+c*6e4,t,n).altitude;s<-.83&&l>=-.83&&(r=c/60),s>=-.83&&l<-.83&&(i=c/60),l>o&&(o=l,a=c/60),s=l}return{rise:r,set:i,noon:a,highest:o}}const Dp=[`New moon`,`Waxing crescent`,`First quarter`,`Waxing gibbous`,`Full moon`,`Waning gibbous`,`Last quarter`,`Waning crescent`];function Op(e){let t=(e%360+360)%360;return t<8||t>352?Dp[0]:t<82?Dp[1]:t<98?Dp[2]:t<172?Dp[3]:t<188?Dp[4]:t<262?Dp[5]:t<278?Dp[6]:Dp[7]}const kp=`${mt}
layout(std140) uniform U { vec4 r0; vec4 r1; vec4 r2; vec4 look; } u;
`,Ap={vertex:`${kp}
layout(location = 0) in vec4 a;
layout(location = 1) in vec4 b;
out vec2 v_corner;
out vec3 v_col;
vec2 starCorner(int v) {
  return vec2(mix(-1.0, 1.0, float(v == 1 || v == 2 || v == 4)), mix(-1.0, 1.0, float(v == 2 || v == 4 || v == 5)));
}
vec3 starColour(float bv) {
  float t = clamp((bv + 0.3) / 2.0, 0.0, 1.0);
  return mix(mix(vec3(0.62, 0.72, 1.0), vec3(1.0, 0.97, 0.92), smoothstep(0.0, 0.35, t)), vec3(1.0, 0.68, 0.42), smoothstep(0.35, 1.0, t));
}
void main() {
  vec3 e = a.xyz;
  vec3 d = vec3(dot(u.r0.xyz, e), dot(u.r1.xyz, e), dot(u.r2.xyz, e));
  float lum = clamp(pow(2.512, u.look.y - a.w) / pow(2.512, u.look.y), 0.0, 1.0);
  float k = u.look.x * smoothstep(0.0, 0.12, d.y) * smoothstep(-0.05, 0.25, u.look.y - a.w);
  float px = max(1.0, u.look.z * pow(lum, 0.25)) * u.look.w * frame.size.z;
  vec2 corner = starCorner(gl_VertexID);
  vec4 c = frame.viewProj * vec4(d, 0.0);
  vec2 off = corner * px / frame.fbsize.xy * c.w;
  gl_Position = clipOut(vec4(c.xy + off, c.w * 0.99999, c.w));
  v_corner = corner;
  v_col = starColour(b.x) * min(1.8, 0.35 + 2.2 * pow(lum, 0.5)) * k;
  if (k <= 0.0 || d.y < -0.02) {
    gl_Position = vec4(0.0, 0.0, 2.0, 1.0);
  }
}
`,fragment:`${kp}
in vec2 v_corner;
in vec3 v_col;
out vec4 outColor;
void main() {
  float r = dot(v_corner, v_corner);
  float a = exp(-r * 2.4);
  outColor = vec4(themed(v_col, 0.0) * a, a);
}
`},jp=`${nn}
// r0..r2: the rows of the turn from the equatorial frame into the view's (x east, y up, z south) · look: (how bright
// 0..1: night, cloud, morph; the faintest magnitude drawn; CSS px a magnitude-0 star spans; a size scale)
struct U { r0: vec4f, r1: vec4f, r2: vec4f, look: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct I { @builtin(vertex_index) vid: u32, @location(0) a: vec4f, @location(1) b: vec4f }
struct O { @builtin(position) pos: vec4f, @location(0) corner: vec2f, @location(1) col: vec3f }
// the six corners of its two triangles
fn starCorner(v: i32) -> vec2f {
  return vec2f(select(-1.0, 1.0, v == 1 || v == 2 || v == 4), select(-1.0, 1.0, v == 2 || v == 4 || v == 5));
}
// a colour index (B−V) as a colour: blue-white hot stars to orange cool ones
fn starColour(bv: f32) -> vec3f {
  let t = clamp((bv + 0.3) / 2.0, 0.0, 1.0);
  return mix(mix(vec3f(0.62, 0.72, 1.0), vec3f(1.0, 0.97, 0.92), smoothstep(0.0, 0.35, t)), vec3f(1.0, 0.68, 0.42), smoothstep(0.35, 1.0, t));
}
@vertex fn vs(i: I) -> O {
  let e = i.a.xyz;
  let d = vec3f(dot(u.r0.xyz, e), dot(u.r1.xyz, e), dot(u.r2.xyz, e));
  // brighter by 2.512 a magnitude, the faintest drawn just visible; low ones dim through the air
  let lum = clamp(pow(2.512, u.look.y - i.a.w) / pow(2.512, u.look.y), 0.0, 1.0);
  let k = u.look.x * smoothstep(0.0, 0.12, d.y) * smoothstep(-0.05, 0.25, u.look.y - i.a.w);
  let px = max(1.0, u.look.z * pow(lum, 0.25)) * u.look.w * frame.size.z;
  var o: O;
  let corner = starCorner(i32(i.vid));
  let c = frame.viewProj * vec4f(d, 0.0);
  // at infinity, behind everything; a few pixels across
  let off = corner * px / frame.fbsize.xy * c.w;
  o.pos = clipOut(vec4f(c.xy + off, c.w * 0.99999, c.w));
  o.corner = corner;
  o.col = starColour(i.b.x) * min(1.8, 0.35 + 2.2 * pow(lum, 0.5)) * k;
  if (k <= 0.0 || d.y < -0.02) {
    o.pos = vec4f(0.0, 0.0, 2.0, 1.0);
  }
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let r = dot(i.corner, i.corner);
  let a = exp(-r * 2.4);
  return vec4f(themed(i.col, 0.0) * a, a);
}
`;function Mp(e){let t=[];for(let n of e.split(`
`)){if(!n||n[0]===`#`)continue;let e=n.split(`	`);if(e.length<3)continue;let r=Number(e[0]),i=Number(e[1]),a=Number(e[2]),o=e[3]!==void 0&&e[3].trim()!==``?Number(e[3]):NaN;e[0].trim()&&e[2].trim()&&Number.isFinite(r)&&Number.isFinite(i)&&Number.isFinite(a)&&t.push({ra:r,dec:i,mag:a,bv:Number.isFinite(o)?o:void 0})}return t}const Np={hipparcos(e={}){return{id:`hipparcos`,attribution:`Stars: Hipparcos (ESA 1997), CDS VizieR`,async load({magnitude:t,loader:n}){let r=e.url??`https://vizier.cds.unistra.fr/viz-bin/asu-tsv?-source=I/239/hip_main&-out=RAICRS&-out=DEICRS&-out=Vmag&-out=B-V&Vmag=%3C${t.toFixed(1)}&-out.max=60000`,i=n?await n.get(r,{kind:`text`,priority:`later`}):await(await fetch(r)).text();if(typeof i!=`string`)throw new K(`E-DATA-002`,`stars: ${r} gave nothing`,{where:`sky.stars`,fix:`check the url (or the network): catalogues.hipparcos({ url })`});return Mp(i).filter(e=>e.mag<=t)}}},custom(e){return{id:e.id??`custom`,attribution:e.attribution??``,load:async({magnitude:t})=>(await e.load(t)).filter(e=>e.mag<=t)}}},Pp=Math.PI/180;function Fp(e){let t=[...e].sort((e,t)=>e.mag-t.mag),n=new Float32Array(t.length*8);return t.forEach((e,t)=>{let r=e.ra*Pp,i=e.dec*Pp;n.set([Math.cos(i)*Math.cos(r),Math.cos(i)*Math.sin(r),Math.sin(i),e.mag,e.bv??.6,0,0,0],t*8)}),n}function Ip(e){return((280.46061837+360.98564736629*((typeof e==`number`?e:e.getTime())/864e5+2440587.5-2451545))%360+360)%360*Pp}function Lp(e,t,n){let r=Ip(e)+t*Pp,i=Math.cos(r),a=Math.sin(r),o=Math.cos(n*Pp),s=Math.sin(n*Pp),c=[i,a,0],l=[-a,i,0],u=[0,0,1];return[l,c.map((e,t)=>o*e+s*u[t]),c.map((e,t)=>s*e-o*u[t])]}const Rp={cover:.18,overcast:0,rain:0,turbidity:2.5,fog:1,wind:6,lightning:0},zp={clear:{label:`Clear`,...Rp},partly:{label:`Partly cloudy`,cover:.48,overcast:.12,rain:0,turbidity:4.5,fog:1.25,wind:9,lightning:0},overcast:{label:`Overcast`,cover:.8,overcast:.62,rain:0,turbidity:8,fog:1.9,wind:12,lightning:0},drizzle:{label:`Drizzle`,cover:.86,overcast:.74,rain:.22,turbidity:9,fog:2.6,wind:12,lightning:0},rain:{label:`Rain`,cover:.94,overcast:.86,rain:.65,turbidity:10,fog:3.6,wind:16,lightning:0},storm:{label:`Storm`,cover:1,overcast:.95,rain:1,turbidity:10,fog:5,wind:26,lightning:1},fog:{label:`Fog`,cover:.9,overcast:.7,rain:0,turbidity:10,fog:14,wind:2,lightning:0}},Bp=e=>4e4/Math.max(1,e.fog)**1.6,Vp=(e,t,n)=>{let r=Math.max(0,Math.min(1,(n-e)/(t-e)));return r*r*(3-2*r)},Hp=(e,t,n)=>[e[0]+(t[0]-e[0])*n,e[1]+(t[1]-e[1])*n,e[2]+(t[2]-e[2])*n],Up=e=>[parseInt(e.slice(1,3),16)/255,parseInt(e.slice(3,5),16)/255,parseInt(e.slice(5,7),16)/255],Wp=e=>e.map(e=>e<=.0031308?e*12.92:1.055*e**(1/2.4)-.055),Gp=Up(`#b4cbe4`),Kp=Up(`#f0c49a`),qp=Up(`#6b5d73`),Jp=Up(`#0d1626`),Yp=[.58,.61,.64],Xp=[.3,.32,.35],Zp=[.012,.018,.04],Qp=[1,.55,.3],$p=[1,.96,.9];function em(e,t,n,r=Rp,i={}){let a=Cp(e,n,t),o=wp(e,n,t),s=Tp(a),c=Tp(o),l=r,u=s[1],d=Vp(-.1,.12,u),f=(1-Vp(.02,.35,u))*Vp(-.08,.02,u),p=1-Vp(-.2,-.02,u),m=Vp(-.22,-.02,u)*(1-d),h=u>.25?Gp:u>0?Hp(Kp,Gp,u/.25):u>-.12?Hp(qp,Kp,(u+.12)/.12):Hp(Jp,qp,Math.max(0,Math.min(1,(u+.25)/.13))),g=Hp(Yp,Xp,l.rain).map(e=>e*(.08+.92*d));h=Hp(h,Wp(g),l.overcast*.85);let _=i.sun===!1?0:d*(1-.82*l.overcast),v=i.moon===!1?0:p*Vp(-.05,.2,c[1])*(1-.8*l.overcast)*(.15+.85*o.illumination),y=Hp(Qp,$p,Vp(.02,.5,u)),b=.74*_*(i.sunlight??1),x=Hp(h,[.85,.9,1],.3),S=.1+.3*d*(1-.25*l.overcast)+.1*m+.2*f*(1-.5*l.overcast),C=_<.02&&v>0,w=Hp([.05,.06,.1],x.map(e=>e*S),d),T=e=>.299*e[0]+.587*e[1]+.114*e[2],E=T(w)+T(y)*b*Math.max(0,u),D=C?1:1+(Math.min(1.7,Math.max(1,(.4+.55*d)/Math.max(.05,E)))-1)*d,O={dir:C?c:s,color:C?[.55*.2*v,.124*v,.8*.2*v]:[y[0]*b*D,y[1]*b*D,y[2]*b*D],ambient:w.map(e=>e*D),bounce:.35*d,daylight:Math.max(.12,d*(1-.45*l.overcast)),shadows:!C&&_>.05&&u>.03,haze:h,visibility:Bp(l),wet:Math.min(1,l.rain*1.4),cover:l.cover,wind:l.wind,rain:l.rain};return{sun:a,moon:o,sunDir:s,moonDir:c,day:d,golden:f,night:p,dusk:m,weather:l,haze:h,light:O}}const tm=e=>e.map(e=>e<=.04045?e/12.92:((e+.055)/1.055)**2.4);var nm=class extends D{kind=`sky`;prog=null;ubuf=null;u=/* @__PURE__ */ new Float32Array(48);now=null;nowKey=``;wNow;wTarget=null;wEase=4;wName=`clear`;nextFlash=5;lastMs=-1;cloudOff=[0,0];constructor(e={}){super({id:`sky`,...e}),this.wNow={...Rp,...typeof e.weather==`object`?e.weather:{}}}setWeather(e,t={}){let n=typeof e==`string`?zp[e]:{...this.wNow,...e};if(!n)throw Error(`no weather '${e}': ${Object.keys(zp).join(`, `)}`);return typeof e==`string`&&(this.wName=e),this.wTarget={...n},this.wEase=t.seconds??4,this.wEase<=0&&(this.wNow={...this.wTarget},this.wTarget=null),(this.attached?.world)?.invalidate?.(),this}get weatherName(){return this.wName}dimNow=0;dimFrom=null;dimTarget(e){let t=this.props.dim;if(!t)return 0;let n=typeof t.amount==`function`?t.amount(e):t.amount;return Math.max(0,Math.min(1,Number(n)||0))}dimColor(){return this.props.dim?Up(this.props.dim.color??`#151c28`):this.dimFrom??Up(`#151c28`)}loader=null;async attach(e){await super.attach(e),this.loader=e.loader;let t=e.graphics;t&&(this.ubuf=t.buffer(`uniform`,192,`earth sky`),this.prog=t.program({label:`earth sky`,wgsl:xp,glsl:bp,blocks:[`Frame`,`U`],depth:{write:!1,test:`always`}}),await this.prog.ready)}weather(e){let t=this.props.weather;return typeof t==`function`?{...Rp,...t(e)}:this.wNow}state(e){let t=this.props.at&&this.props.at!==`view`?this.props.at:[e.view.lon,e.view.lat],n=this.weather(e),r=`${e.time.now.getTime()}|${t[0].toFixed(2)}|${t[1].toFixed(2)}|${Object.values(n).join(`,`)}|${this.props.sunlight??1}`;return(r!==this.nowKey||!this.now)&&(this.nowKey=r,this.now=em(e.time.now,t[0],t[1],n,{sun:this.props.sun,moon:this.props.moon,sunlight:this.props.sunlight})),this.now}light(e){let t=this.state(e).light,n=this.props.haze,r=this.props.clouds,i=n?.clear??4e4,a=(t.visibility??4e4)*i/4e4/Math.max(.05,n?.level??1),o=Math.max(0,e.view.altitude-(n?.base??0)),s={...t,visibility:a*(1+o/Math.max(1,n?.height??600)),flash:this.flash};if(this.dimNow>0&&t.haze&&(s.haze=Hp(t.haze,this.dimColor(),this.dimNow*.85)),r!==!1&&r?.shadows!==!1&&(t.cover??0)>.01){let t=J*Math.cos(e.view.lat*G);s.clouds={base:r?.base??2700,x:((t*e.view.lon*G+this.cloudOff[0])%32e3+32e3)%32e3,z:((-t*Math.log(Math.tan(Math.PI/4+e.view.lat*G/2))+this.cloudOff[1])%32e3+32e3)%32e3}}return s}feedNow=null;feedAt=-1e9;feedKey=``;coverAt=-1e9;coverFrom=null;feed(e){return this.feedNow=e,this.feedAt=-1e9,this.feedKey=``,this}readFeeds(e,t){let n=this.feedNow;if(n&&t-this.feedAt>(n.every??10)*1e3){this.feedAt=t;let r=typeof n.data==`function`?n.data():n.data.features(),i=[e.view.lon,e.view.lat],a=Math.cos(i[1]*G),o=r.flatMap(e=>e.geometry?.type===`Point`?[{p:e.props,d:Math.hypot((e.geometry.coordinates[0]-i[0])*a,e.geometry.coordinates[1]-i[1])}]:[]).sort((e,t)=>e.d-t.d).slice(0,4),s=e=>{if(!e)return null;let t=0,n=0;for(let r of o){let i=typeof e==`function`?e(r.p):Number(r.p[e]);if(i===null||!Number.isFinite(i))continue;let a=1/Math.max(1e-6,r.d)**2;t+=a,n+=a*i}return t>0?n/t:null},c=s(n.wind),l=s(n.rain),u=s(n.visibility),d=s(n.cover),f={};if(c!==null&&(f.wind=Math.max(0,c)),l!==null&&(f.rain=Math.max(0,Math.min(1,l/8)),l>25&&(f.lightning=1)),u!==null&&(f.fog=Math.max(1,(4e4/Math.max(50,u))**(1/1.6))),d!==null){let e=Math.max(0,Math.min(1,d>1?d/100:d));f.cover=e,f.overcast=Math.max(0,(e-.45)*1.6)}let p=JSON.stringify(f);p!==this.feedKey&&Object.keys(f).length&&(this.feedKey=p,this.setWeather(f,{seconds:n.seconds??3}),this.wName=`feed`)}let r=this.props.clouds===!1?void 0:this.props.clouds,i=r?.source;i&&this.loader&&t-this.coverAt>1e4&&(this.coverAt=t,rm(i,e.view.lon,e.view.lat,r?.zoom??6,this.loader).then(e=>{e!==null&&Number.isFinite(e)&&e!==this.coverFrom&&(this.coverFrom=e,this.setWeather({cover:Math.max(0,Math.min(1,e))},{seconds:3}))}).catch(()=>{}))}readouts(){let e=()=>this.now;return{sun:()=>{let t=e();return t?{altitude:+t.sun.altitude.toFixed(2),azimuth:+t.sun.azimuth.toFixed(2)}:null},moon:()=>{let t=e();return t?{altitude:+t.moon.altitude.toFixed(2),azimuth:+t.moon.azimuth.toFixed(2),illumination:+t.moon.illumination.toFixed(3),phase:Op(t.moon.elongation)}:null},daylight:()=>e()?.light.daylight??null}}tickWeather(e){(this.feedNow||this.props.clouds!==!1&&this.props.clouds?.source)&&this.readFeeds(e.s,e.now);let t=e.now,n=this.lastMs<0?0:Math.max(0,Math.min(.25,(t-this.lastMs)/1e3));if(this.lastMs=t,this.wTarget){let e=1-Math.exp(-n*(3/Math.max(.1,this.wEase))),t=!0;for(let n of Object.keys(this.wNow)){let r=this.wTarget[n]-this.wNow[n];this.wNow[n]+=r*e,Math.abs(r)>.001&&(t=!1)}t&&(this.wNow={...this.wTarget},this.wTarget=null)}let r=this.weather(e.s);r.lightning>.5&&(this.nextFlash-=n,this.nextFlash<0&&(this.flash=1,this.nextFlash=3+9*((Math.sin(t*.001237)+1)/2))),this.flash>0&&(this.flash=Math.max(0,this.flash-n*4)),this.cloudOff[0]+=n*r.wind,this.cloudOff[1]+=n*r.wind*.35;let i=this.props.dim?null:e.world?.themeNow?.sky??null;if(this.props.dim){let t=this.dimTarget(e.s),r=1-Math.exp(-n*(3/Math.max(.05,this.props.dim?.seconds??1)));this.dimNow=Math.abs(t-this.dimNow)<.001?t:this.dimNow+(t-this.dimNow)*r}else(i||this.dimNow>0)&&(this.dimNow=i?i.amount:0,this.dimFrom=i?[i.color[0],i.color[1],i.color[2]]:null)}starsProg=null;starsU=null;starsBuf=null;starsN=0;starsLoad=null;starsKey=``;starsFailed=null;su=/* @__PURE__ */ new Float32Array(16);stars(e){return this.props={...this.props,stars:e},(this.attached?.world)?.invalidate?.(),this}get starsOpts(){let e=this.props.stars;return!e||typeof e!=`object`?null:{magnitude:e.magnitude??6,catalogue:e.catalogue??(this.defaultCatalogue??=Np.hipparcos()),brightness:e.brightness??1,size:e.size??3}}defaultCatalogue=null;loadStars(e){let t=`${e.catalogue.id}|${e.magnitude}`,n=this.attached?.graphics;if(!n||t===this.starsKey)return;this.starsKey=t,this.starsFailed=null,this.starsProg??=n.program({label:`stars`,wgsl:jp,glsl:Ap,blocks:[`Frame`,`U`],instance:{stride:8,attributes:[{name:`a`,location:0,size:4,offset:0},{name:`b`,location:1,size:4,offset:4}]},blend:`add`,depth:{write:!1,test:`always`},cull:`none`}),this.starsU??=n.buffer(`uniform`,64,`stars`);let r=this.starsLoad=Promise.all([e.catalogue.load({magnitude:e.magnitude,loader:this.loader}),this.starsProg.ready]).then(([e])=>{if(this.starsLoad!==r||!this.attached?.graphics)return;let t=Fp(e);this.starsBuf?.destroy(),this.starsBuf=t.length?n.buffer(`instance`,t,`stars`):null,this.starsN=t.length/8,this.attached.world.invalidate?.()},e=>{this.starsLoad===r&&(this.starsFailed=String(e?.message??e),this.attached?.world.error(e))}).finally(()=>{this.starsLoad===r&&(this.starsLoad=null)})}status(){let e=super.status();return this.starsLoad&&e.pending.push(`stars loading`),this.starsFailed&&e.failed.push({what:`stars`,error:this.starsFailed}),e}attribution(){let e=this.starsOpts;return e&&this.starsN?[e.catalogue.attribution].filter(Boolean):[]}frame(e){if(!this.prog||!this.ubuf)return[];this.tickWeather(e);let t=this.state(e.s),n=this.props,r=this.u,i=t.weather,a=t.sunDir[1],o=Vp(-.03,.02,a),s=1-Vp(0,.35,a),c=this.dimNow,l=c>0?this.dimColor():null,u=Hp(Hp(Yp,Xp,i.rain).map(e=>e*(.08+.92*t.day)),Zp,t.night*(1-i.overcast*.5)),d=Wp(l?Hp(u,tm(l),c):u),f=(.78+.22*(1-t.day))*(1+.2*i.overcast*t.day)*.9;r.set([t.sunDir[0],t.sunDir[1],t.sunDir[2],n.sun===!1?0:(1-i.overcast)*o],0),r.set([t.moonDir[0],t.moonDir[1],t.moonDir[2],n.moon===!1?0:Vp(-.03,.03,t.moonDir[1])*(.35+.65*t.night)*(1-.92*i.overcast)*(1-.5*i.cover*t.day)],4),r.set([i.turbidity,(2.2+1.5*t.golden)*(1-.5*i.overcast),.004+.016*i.overcast,.82],8);let p=this.starsOpts,m=t.night*(1-Vp(.3,.9,i.cover)),h=p||n.stars===!1?0:typeof n.stars==`number`?n.stars:1;r.set([Math.max(Math.min(.97,i.overcast*.9+t.night*.85),c*.95),t.day*(1-i.overcast)*(1-.6*t.golden)*(1-c),h*m,e.camera.morph],12);let g=this.flash>0?this.flash*(.6+.4*Math.sin(this.flash*40)):0;r.set([d[0],d[1],d[2],g],16);let _=l?Hp(t.haze,l,c*.85):t.haze;r.set([_[0],_[1],_[2],f],20),r.set([...n.space??[.015,.02,.04],1],24),r.set([...n.glow??[.55,.75,1],1],28),r.set([J,n.sun===!1?0:o*(.55+.35*s)*(1-.75*i.overcast)*(1-.8*i.rain),.0095,0],32);let v=e.camera,y=n.clouds===!1?-1:n.clouds?.base??2700,b=J*Math.cos(v.lat*G),x=((b*v.lon*G+this.cloudOff[0])%32e3+32e3)%32e3,S=((-b*Math.log(Math.tan(Math.PI/4+v.lat*G/2))+this.cloudOff[1])%32e3+32e3)%32e3;r.set([y<0?0:i.cover,x,S,y<0?-1:y-0],36);let C=t.day*(1-.82*i.overcast),w=Hp(Hp([1,.55,.3],[1,.96,.9],Vp(.02,.5,a)).map(e=>e*(.03+.95*C+.5*t.dusk)),t.haze,.25).map((e,n)=>e+[.05,.06,.09][n]*t.night),T=Hp(Hp(Yp,Xp,i.rain).map(e=>e*(.08+.92*t.day)*(.55+.4*t.day)),t.haze,.3);r.set([...Wp(w),1-Vp(3500,9e3,v.groundY+v.dist*Math.sin(v.pitch))],40),r.set([...Wp(T),0],44),this.ubuf.write(r);let E=[{program:this.prog,blocks:[e.frame,this.ubuf],count:3}],D=p?p.brightness*m*e.camera.morph:0;if(p&&D>.003&&(this.loadStars(p),this.starsBuf&&this.starsN&&this.starsProg&&this.starsU)){let t=n.at&&n.at!==`view`?n.at:[e.s.view.lon,e.s.view.lat],r=Lp(e.s.time.now,t[0],t[1]),i=this.su;i.set([...r[0],0,...r[1],0,...r[2],0,D,p.magnitude,p.size,1]),this.starsU.write(i),E.push({program:this.starsProg,blocks:[e.frame,this.starsU],instances:this.starsBuf,count:6,instanceCount:this.starsN})}return E}flash=0;animating(e){let t=this.weather(e);return!!this.props.dim&&Math.abs(this.dimTarget(e)-this.dimNow)>.001||this.flash>0||this.wTarget!==null||t.lightning>.5||t.cover>.05&&t.wind>0&&this.cloudsMove||Math.abs(e.time.rate??0)>30}wantsFrames(e){let t=this.weather(e);if(this.props.dim&&Math.abs(this.dimTarget(e)-this.dimNow)>.001||this.flash>0||this.wTarget!==null)return 1/0;let n=0;return t.lightning>.5&&(n=10),this.cloudsMove&&t.cover>.05&&t.wind>0&&(n=Math.max(n,Sn(t.wind/Math.max(.001,e.view.pixelSize),{stepPx:6}))),n}cloudsMove=!1;detach(){this.prog?.destroy(),this.ubuf?.destroy(),this.prog=null,this.ubuf=null,this.starsProg?.destroy(),this.starsU?.destroy(),this.starsBuf?.destroy(),this.starsProg=this.starsU=this.starsBuf=null,this.starsN=0,this.starsKey=``,this.starsLoad=null,super.detach()}};async function rm(e,t,n,r,i){if(typeof OffscreenCanvas>`u`)return null;let a=Math.max(e.minZoom,Math.min(e.maxZoom,r)),[o,s]=T(t,n,a),c=2**a,l=new OffscreenCanvas(256,256).getContext(`2d`,{willReadFrequently:!0});if(!l||!await e.draw(l,{z:a,x:o,y:s},{loader:i,priority:`later`,prefetch:!1}))return null;let u=(t+180)/360*c-o,d=(1-Math.log(Math.tan(Math.PI/4+n*G/2))/Math.PI)/2*c-s,f=l.getImageData(Math.min(255,Math.max(0,Math.floor(u*256))),Math.min(255,Math.max(0,Math.floor(d*256))),1,1).data;return f[3]/255*((.299*f[0]+.587*f[1]+.114*f[2])/255)}const im=(e={})=>new nm(e),am=110574,om=/* @__PURE__ */ new WeakMap,sm=/* @__PURE__ */ new WeakMap;var cm=class extends Zt{stands=!1;index=null;avail=/* @__PURE__ */ new Set;slots=/* @__PURE__ */ new Map;inflight=0;extras=/* @__PURE__ */ new Map;failure2=null;skipped=0;constructor(e){super({id:`street`,...e})}async attach(e){await super.attach(e);let t=this.props.url;this.index=typeof t==`string`?await e.loader.get(t,{kind:`json`,priority:`soon`}):t,this.index?.tiles?this.avail=new Set(this.index.tiles.map(([e,t])=>`${e}_${t}`)):this.failure2=`no tile index`}urlOf(e,t){return this.props.tileUrl?this.props.tileUrl(e,t):`${(typeof this.props.url==`string`?this.props.url:``).replace(/[^/]*$/,``)}${e}_${t}.json`}ctxOf(e,t,n){let r=this.index,i=111320*Math.cos(r.origin[1]*G),a=[e*r.size,t*r.size],o=(e,t)=>[r.origin[0]+(a[0]+e)/i,r.origin[1]-(a[1]+t)/am];return{ix:e,iz:t,size:r.size,origin:r.origin,at:a,lonLat:o,groundAt:(e,t)=>{let[r,i]=o(e,t);return n.heightAt(r,i)}}}built(){return[...this.extras].map(([e,t])=>({key:e,...t}))}tiles(){let e=[];for(let[t,n]of this.slots){let r=this.extras.get(t);n.state===`built`&&n.tile&&r&&e.push({key:t,tile:n.tile,ctx:r.ctx})}return e}builtTile(e,t){let n=this.slots.get(`${e}_${t}`);return n?.state===`built`&&n.tile&&this.extras.has(`${e}_${t}`)?n.tile:null}rebuild(){for(let e of this.slots.values())(e.state===`built`||e.state===`building`||e.state===`arrived`)&&e.tile&&(e.state=`ready`,e.got=void 0);return(this.attached?.world)?.invalidate?.(),this}get area(){return this.index}frame(e){return this.stream(e),super.frame(e)}stream(e){let t=this.index;if(!t||!this.attached)return;let n=e.camera,r=this.props.near??4e3,i=111320*Math.cos(t.origin[1]*G),a=(n.lon-t.origin[0])*i,o=-(n.lat-t.origin[1])*am,s=t.size,c=n.dist>r*1.3,l=Math.max(700,Math.min(2600,n.dist/n.frameK*1.4)),u=/* @__PURE__ */ new Map;if(n.dist<=r&&n.morph>.98)for(let e=Math.floor((a-l)/s);e<=Math.floor((a+l)/s);e++)for(let t=Math.floor((o-l)/s);t<=Math.floor((o+l)/s);t++){let n=`${e}_${t}`;if(!this.avail.has(n))continue;let r=Math.hypot((e+.5)*s-a,(t+.5)*s-o);r<=l+s*.7&&u.set(n,r)}for(let[e,t]of this.slots){if(u.has(e))continue;let[r,i]=e.split(`_`).map(Number),l=Math.hypot((r+.5)*s-a,(i+.5)*s-o);c||l>Math.max(1200,n.dist*1.8)+s?(this.slots.delete(e),t.state===`built`&&(this.drop(e),this.extras.delete(e),this.props.onTile?.(e,null,null))):(t.state===`ready`||t.state===`missing`||t.state===`arrived`)&&this.slots.delete(e)}let d=[...u].sort((e,t)=>e[1]-t[1]);for(let[e,t]of d){let n=this.slots.get(e);if(n){n.dist=t;continue}if(this.inflight>=(this.props.fetches??6))continue;let[r,i]=e.split(`_`).map(Number),a={state:`loading`,dist:t};this.slots.set(e,a),this.inflight++,this.attached.loader.get(this.urlOf(r,i),{kind:`json`,priority:`now`}).then(t=>{this.slots.get(e)===a&&(a.tile=t??void 0,a.state=t?`ready`:`missing`)},()=>{this.slots.get(e)===a&&(a.state=`missing`)}).finally(()=>{this.inflight--,(this.attached?.world)?.invalidate?.()})}let f=(e,n,r)=>{this.put(e,{...n.chunk,origin:t.origin,at:r.at}),this.extras.set(e,{extra:n.extra,ctx:r}),this.props.onTile?.(e,n.extra??null,r)},p=this.props.putMs??3,m=e.n??e.now,h=om.get(e.world)??{n:NaN,ms:0,puts:0};h.n!==m&&(h.n=m,h.ms=0,h.puts=0,om.set(e.world,h));let g=0,_=!1;for(let[t]of d){let n=this.slots.get(t);if(!n||n.state!==`arrived`||!n.got)continue;if(_=!0,h.puts&&h.ms>=p&&(g||this.skipped<2))break;let{b:r,ctx:i}=n.got,a=performance.now();n.got=void 0,n.state=`built`,g++;try{f(t,r,i)}catch(t){e.world.error(t)}h.ms+=performance.now()-a,h.puts++}this.skipped=_&&!g?this.skipped+1:0;let v=performance.now(),y=this.props.budgetMs??8,b=this.props.worker,x=b?Math.max(2,b.size*2):4,S=0;for(let e of this.slots.values())e.state===`building`&&S++;for(let[t]of d){if(performance.now()-v>y||(b?(sm.get(b)??0)>=x:S>=4))break;let n=this.slots.get(t);if(!n||n.state!==`ready`||!n.tile)continue;let[r,i]=t.split(`_`).map(Number),a=this.ctxOf(r,i,e.world);try{let r=this.props.build(n.tile,a);if(r instanceof Promise){n.state=`building`,S++,b&&sm.set(b,(sm.get(b)??0)+1),r.finally(()=>{b&&sm.set(b,(sm.get(b)??1)-1)}).catch(()=>{}),r.then(e=>{this.slots.get(t)===n&&n.state===`building`&&(n.got={b:e,ctx:a},n.state=`arrived`)},r=>{e.world.error(r),this.slots.get(t)===n&&n.state===`building`&&(n.state=`built`)}).finally(()=>(this.attached?.world)?.invalidate?.());continue}f(t,r,a)}catch(t){e.world.error(t)}n.state=`built`}}status(){let e=super.status();if(this.failure2&&e.failed.push({what:this.id,error:this.failure2}),this.shown<=0&&this.target<=0)return e;let t=0,n=0;for(let e of this.slots.values())e.state===`loading`&&t++,(e.state===`ready`||e.state===`building`||e.state===`arrived`)&&n++;return t&&e.pending.push(`${t} tile(s) loading`),n&&e.pending.push(`${n} tile(s) building`),e.ready=!e.pending.length,e}animating(){if(super.animating())return!0;for(let e of this.slots.values())if(e.state===`ready`||e.state===`building`||e.state===`arrived`)return!0;return!1}attribution(){return this.index?.attribution?[this.index.attribution]:[]}tileCounts(){let e=0,t=0,n=0;for(let r of this.slots.values())r.state===`loading`?e++:r.state===`ready`?t++:r.state===`built`&&n++;return{loading:e,ready:t,built:n,available:this.avail.size}}};const lm=e=>new cm(e);function um(e,t,n){let r=t.lanes??n.lanes(e),i=t.width??n.carriageway(e),a=t.reserve??{left:null,right:null},o=a.left!==null||a.right!==null,s=t=>a[t]??a[t===`left`?`right`:`left`]??n.reserve(e)/2,c=Math.min(s(`left`),s(`right`));!n.noVerge(e)&&c-n.kerb-.3<i/2&&(i=Math.max(3,2*(c-n.kerb-.3)));let l=r=>{if(n.noVerge(e))return{kerb:0,strip:0,footpath:0,gap:0};let a=s(r)-i/2,o=n.kerb,c=t.sidewalk?.[r]===!1?0:t.footpath?.[r]??n.footpath(e),l=n.gap(e),u=a-o-c-l,d=c;return u<n.minStrip&&(l=Math.max(0,Math.min(l,a-o-c)),u=Math.max(0,a-o-d-l),u<n.minStrip&&(d=Math.max(0,a-o-l),u=0)),{kerb:o,strip:u,footpath:d,gap:l}};return{carriageway:i,lanes:r,median:Math.min(n.median(e),Math.max(0,i-6.6*Math.max(1,Math.floor(r/2)))),left:l(`left`),right:l(`right`),reserve:{left:a.left,right:a.right},from:o?`reserve`:t.fromTags?`tags`:`class`}}function dm(e,t,n={}){let r=n.reach??40,i=[],a=[],o=1/0,s=-1/0,c=1/0,l=-1/0;for(let[t,n]of e)o=Math.min(o,t),s=Math.max(s,t),c=Math.min(c,n),l=Math.max(l,n);let u=t.filter(e=>{let t=1/0,n=-1/0,i=1/0,a=-1/0;for(let[r,o]of e)t=Math.min(t,r),n=Math.max(n,r),i=Math.min(i,o),a=Math.max(a,o);return n>=o-r&&t<=s+r&&a>=c-r&&i<=l+r});if(!u.length)return{left:null,right:null};let d=Math.max(1,Math.floor(e.length/12));for(let t=0;t+1<e.length;t+=d){let[n,o]=e[t],[s,c]=e[t+1],l=Math.hypot(s-n,c-o);if(l<.5)continue;let d=(n+s)/2,f=(o+c)/2,p=(s-n)/l,m=(c-o)/l,h=u.find(e=>fm(d,f,e));if(h)for(let[e,t,n]of[[m,-p,i],[-m,p,a]]){let i=pm(d,f,e,t,h,r);i!==null&&n.push(i)}}let f=e=>e.length?e.sort((e,t)=>e-t)[e.length>>1]:null;return{left:f(i),right:f(a)}}function fm(e,t,n){let r=!1;for(let i=0,a=n.length-1;i<n.length;a=i++){let[o,s]=n[i],[c,l]=n[a];s>t!=l>t&&e<(c-o)*(t-s)/(l-s)+o&&(r=!r)}return r}function pm(e,t,n,r,i,a){let o=null;for(let s=0,c=i.length-1;s<i.length;c=s++){let[l,u]=i[c],[d,f]=i[s],p=d-l,m=f-u,h=n*m-r*p;if(Math.abs(h)<1e-9)continue;let g=((l-e)*m-(u-t)*p)/h,_=((l-e)*r-(u-t)*n)/h;g>.3&&g<a&&_>=0&&_<=1&&(o===null||g<o)&&(o=g)}return o}const mm=e=>e.kerb+e.strip+e.footpath+e.gap;function hm(e,t){let n=0;for(let r=0;r+1<e.length;r++){let[i,a]=e[r],[o,s]=e[r+1],c=Math.hypot(o-i,s-a);if(n+c>=t||r+2===e.length){let e=c>0?Math.max(0,Math.min(1,(t-n)/c)):0;return{x:i+(o-i)*e,z:a+(s-a)*e,tx:c?(o-i)/c:1,tz:c?(s-a)/c:0}}n+=c}let r=e[0]??[0,0];return{x:r[0],z:r[1],tx:1,tz:0}}const gm=e=>{let t=0;for(let n=0;n+1<e.length;n++)t+=Math.hypot(e[n+1][0]-e[n][0],e[n+1][1]-e[n][1]);return t};function _m(e,t,n){let r=[];if(n<=t)return r;let i=hm(e,t);r.push([i.x,i.z]);let a=0;for(let i=0;i+1<e.length;i++)a+=Math.hypot(e[i+1][0]-e[i][0],e[i+1][1]-e[i][1]),a>t+1e-6&&a<n-1e-6&&r.push([e[i+1][0],e[i+1][1]]);let o=hm(e,n);return r.push([o.x,o.z]),r}const vm={carriageway:e=>[24,21,20,14,11.6,7.3,4,4,3,3][Math.max(0,Math.min(9,e))],lanes:e=>[6,4,4,4,2,2,1,1,1,1][Math.max(0,Math.min(9,e))],reserve:e=>[60,45,41,34,24,16,8,8,6,6][Math.max(0,Math.min(9,e))],footpath:e=>e<=2?3:e<=4?2.5:1.5,median:e=>e===2?6:0,kerb:.45,minStrip:.6,gap:e=>e<=4?.5:.05,noVerge:e=>e<=1||e>=6};function ym(e,t={},n=vm){let r=t.tags??{},i=e=>{let t=parseFloat(e??``);return Number.isFinite(t)&&t>0?t:void 0},a=r.sidewalk??r[`sidewalk:both`],o=e=>(r[`sidewalk:${e}`]??(a===`both`||a===e?`yes`:a===`no`||a===`none`||a&&a!==e&&a!==`both`&&a!==`separate`?`no`:void 0))!==`no`,s=t.lanes??i(r.lanes);return um(e,{width:t.width??i(r.width)??(i(r.lanes)?s*3.3:void 0),lanes:s,reserve:t.reserve,sidewalk:{left:o(`left`),right:o(`right`)},footpath:{left:i(r[`sidewalk:left:width`])??i(r[`sidewalk:width`]),right:i(r[`sidewalk:right:width`])??i(r[`sidewalk:width`])},fromTags:!!(r.lanes||r.width||a||r[`sidewalk:left`]||r[`sidewalk:right`])},n)}function bm(e){let t=0;for(let n=0,r=e.length-1;n<e.length;r=n++)t+=(e[r][0]+e[n][0])*(e[r][1]-e[n][1]);return t/2}function xm(e,t){let n=[];for(let r=0;r+1<e.length;r++){let[i,a]=e[r],[o,s]=e[r+1],c=Math.max(1,Math.ceil(Math.hypot(o-i,s-a)/t));for(let e=0;e<c;e++)n.push([i+(o-i)*e/c,a+(s-a)*e/c])}return e.length&&n.push([e[e.length-1][0],e[e.length-1][1]]),n}function Sm(e){let t={area:1/0,cx:0,cz:0,a:0,b:0,ux:1,uz:0,vx:0,vz:1};for(let n=0;n<e.length;n++){let[r,i]=e[n],[a,o]=e[(n+1)%e.length],s=Math.hypot(a-r,o-i);if(s<.5)continue;let c=(a-r)/s,l=(o-i)/s,u=-l,d=c,f=1/0,p=-1/0,m=1/0,h=-1/0;for(let[t,n]of e){let e=t*c+n*l,r=t*u+n*d;f=Math.min(f,e),p=Math.max(p,e),m=Math.min(m,r),h=Math.max(h,r)}let g=(p-f)*(h-m);if(g<t.area){let e=(f+p)/2,n=(m+h)/2,r=(p-f)/2,i=(h-m)/2;t=r>=i?{area:g,cx:e*c+n*u,cz:e*l+n*d,a:r,b:i,ux:c,uz:l,vx:u,vz:d}:{area:g,cx:e*c+n*u,cz:e*l+n*d,a:i,b:r,ux:u,uz:d,vx:-c,vz:-l}}}return t}function Cm(e,t,n){let r=null;for(let i of n){let n=i.bx-i.ax,a=i.bz-i.az,o=n*n+a*a;if(o<1e-6)continue;let s=Math.max(0,Math.min(1,((e-i.ax)*n+(t-i.az)*a)/o)),c=i.ax+n*s,l=i.az+a*s,u=Math.hypot(e-c,t-l);if(!r||u<r.d){let e=Math.sqrt(o);r={px:c,pz:l,d:u,tx:n/e,tz:a/e,seg:i}}}return r}function wm(e,t,n){let r=!1;for(let i=0,a=n.length-1;i<n.length;a=i++){let[o,s]=n[i],[c,l]=n[a];s>t!=l>t&&e<(c-o)*(t-s)/(l-s)+o&&(r=!r)}return r}var Tm=class{s;constructor(e){this.s=e>>>0||1}next(){let e=this.s+=1831565813;return e=Math.imul(e^e>>>15,e|1),e^=e+Math.imul(e^e>>>7,e|61),((e^e>>>14)>>>0)/4294967296}range(e,t){return e+(t-e)*this.next()}chance(e){return this.next()<e}pick(e){return e[Math.floor(this.next()*e.length)%e.length]}weighted(e){let t=Object.entries(e),n=0;for(let[,e]of t)n+=e;let r=this.next()*n;for(let[e,n]of t)if(r-=n,r<=0)return e;return t[t.length-1][0]}};const Em=(...e)=>{let t=2166136261;for(let n of e)t=Math.imul(t^(n|0),16777619),t=Math.imul(t^(n*1e3|0),16777619);return t>>>0};var Dm=class{rings=[];bands=[];addRing(e){let t=1/0,n=-1/0,r=1/0,i=-1/0;for(let[a,o]of e)t=Math.min(t,a),n=Math.max(n,a),r=Math.min(r,o),i=Math.max(i,o);return this.rings.push({ring:e,x0:t,x1:n,z0:r,z1:i}),this}addBand(e,t,n){for(let r=0;r+1<e.length;r++){let[i,a]=e[r],[o,s]=e[r+1],c=Math.hypot(o-i,s-a);c<.05||this.bands.push({ax:i,az:a,bx:o,bz:s,tx:(o-i)/c,tz:(s-a)/c,len:c,d0:Math.min(t,n),d1:Math.max(t,n)})}return this}bandAt(e,t,n){for(let r of this.bands){let i=(e-r.ax)*r.tx+(t-r.az)*r.tz;if(i<-n||i>r.len+n)continue;let a=-(e-r.ax)*r.tz+(t-r.az)*r.tx;if(a>r.d0-n&&a<r.d1+n)return{b:r,d:a}}return null}blocked(e,t,n=0){if(this.bandAt(e,t,n))return!0;for(let n of this.rings)if(e>=n.x0&&e<=n.x1&&t>=n.z0&&t<=n.z1&&wm(e,t,n.ring))return!0;return!1}place(e,t,n={}){let r=n.r??0,i=n.nudge??2;for(let n of this.rings)if(e>=n.x0&&e<=n.x1&&t>=n.z0&&t<=n.z1&&wm(e,t,n.ring))return null;let a=this.bandAt(e,t,r);if(!a)return[e,t];let{b:o,d:s}=a,c=-o.tz,l=o.tx;for(let n of[o.d0-r-.3,o.d1+r+.3].sort((e,t)=>Math.abs(e-s)-Math.abs(t-s))){if(Math.abs(n-s)>i)continue;let a=e+c*(n-s),o=t+l*(n-s);if(!this.blocked(a,o,r))return[a,o]}return null}};const Om={wings:3,minWing:6,minWidth:2,step:.4},km=e=>(e.x1-e.x0)*(e.z1-e.z0);function Am(e,t,n=jm){let r=Mm;if(r&&r.footprint===e&&r.f0===t?.[0]&&r.f1===t?.[1]&&r.o===n)return r.plan;let i=Nm(e,t,n);return Mm={footprint:e,f0:t?.[0],f1:t?.[1],o:n,plan:i},i}const jm={};let Mm=null;function Nm(e,t,n){let r={...Om,...n},i=Sm(e),a=t?[t[0],t[1]]:[0,1],o=[[i.ux,i.uz],[-i.ux,-i.uz],[i.vx,i.vz],[-i.vx,-i.vz]],s=o[0],c=-1/0;for(let e of o){let t=e[0]*a[0]+e[1]*a[1];t>c+1e-9&&(c=t,s=e)}let l=Math.atan2(-s[0],s[1])*180/Math.PI,u=[s[1],-s[0]],d=e.map(([e,t])=>[e*u[0]+t*u[1],e*s[0]+t*s[1]]),f=Math.abs(bm(d)),p=Bm(d,r),m=p[0]??Im(d),h=p.slice(1).map(e=>({...e,side:zm(e,m)})),g=f>0?Math.min(1,p.reduce((e,t)=>e+km(t),0)/f):0;return{turn:l,x:u,z:s,footprint:d,main:m,wings:h,area:f,cover:g,rectangularity:f/Math.max(1e-9,4*i.a*i.b)}}function Pm(e,t,n=.01){let r={x0:t.x0+n,x1:t.x1-n,z0:t.z0+n,z1:t.z1-n};if(r.x1<=r.x0||r.z1<=r.z0)return!0;let i=e.footprint,a=Math.max(1,Math.ceil((r.x1-r.x0)/.5)),o=Math.max(1,Math.ceil((r.z1-r.z0)/.5));for(let e=0;e<=a;e++){let t=r.x0+(r.x1-r.x0)*e/a;if(!Lm(t,r.z0,i)||!Lm(t,r.z1,i))return!1}for(let e=1;e<o;e++){let t=r.z0+(r.z1-r.z0)*e/o;if(!Lm(r.x0,t,i)||!Lm(r.x1,t,i))return!1}for(let[e,t]of i)if(Rm(e,t,r,0))return!1;return!0}function Fm(e,t,n,r){let i=n===`x0`||n===`z0`?-1:1;for(let a of[1,.75,.5,.25]){let o={...t};if(o[n]+=i*r*a,Pm(e,o))return o}return{...t}}const Im=e=>{let t=1/0,n=-1/0,r=1/0,i=-1/0;for(let[a,o]of e)t=Math.min(t,a),n=Math.max(n,a),r=Math.min(r,o),i=Math.max(i,o);return{x0:t,x1:n,z0:r,z1:i}},Lm=(e,t,n)=>{let r=!1;for(let i=0,a=n.length-1;i<n.length;a=i++){let[o,s]=n[i],[c,l]=n[a];s>t!=l>t&&e<(c-o)*(t-s)/(l-s)+o&&(r=!r)}return r},Rm=(e,t,n,r)=>e>n.x0+r&&e<n.x1-r&&t>n.z0+r&&t<n.z1-r;function zm(e,t){let n=.3,r=Math.min(e.x1,t.x1)-Math.max(e.x0,t.x0),i=Math.min(e.z1,t.z1)-Math.max(e.z0,t.z0);return r>1&&Math.abs(e.z0-t.z1)<n?`front`:r>1&&Math.abs(e.z1-t.z0)<n?`back`:i>1&&Math.abs(e.x1-t.x0)<n?`left`:i>1&&Math.abs(e.x0-t.x1)<n?`right`:`apart`}function Bm(e,t){let n=Im(e),r=n.x1-n.x0,i=n.z1-n.z0;if(!(r>.5&&i>.5))return[];let a=.01,o=n.x0+a,s=n.z0+a,c=Math.max(1,Math.round((r-2*a)/t.step)),l=Math.max(1,Math.round((i-2*a)/t.step)),u=(r-2*a)/c,d=(i-2*a)/l,f=(t,n,r,i,a)=>{let o=[];for(let n=0,r=e.length-1;n<e.length;r=n++){let[i,a]=e[n],[s,c]=e[r];a>t!=c>t&&o.push((s-i)*(t-a)/(c-a)+i)}o.sort((e,t)=>e-t);for(let e=0,t=0;e<n;e++){let n=r+e*u;for(;t<o.length&&o[t]<=n;)t++;i[a+e]=t&1}},p=new Uint8Array((c+1)*(l+1)),m=new Uint8Array(c*l);for(let e=0;e<=l;e++)f(s+e*d,c+1,o,p,e*(c+1));for(let e=0;e<l;e++)f(s+(e+.5)*d,c,o+.5*u,m,e*c);let h=new Uint8Array(c*l),g=(e,t)=>p[t*(c+1)+e];for(let e=0;e<l;e++)for(let t=0;t<c;t++)h[e*c+t]=g(t,e)&&g(t+1,e)&&g(t,e+1)&&g(t+1,e+1)&&m[e*c+t]?1:0;let _=[];for(let n=0;_.length<=t.wings&&n<t.wings+4;n++){let n=!_.length,r=Hm(h,c,l,u,d,n?0:Math.max(0,t.minWidth-1.5*Math.max(u,d)));if(!r)break;let i={x0:o+r.i0*u,x1:o+(r.i1+1)*u,z0:s+r.j0*d,z1:s+(r.j1+1)*d};for(let e of _)i=Vm(i,e);i=Um(i,e,_,u,d);let a=n||km(i)>=t.minWing&&Math.min(i.x1-i.x0,i.z1-i.z0)>=t.minWidth;a&&_.push(i);let f=a?.5:0;for(let e=r.j0-1;e<=r.j1+1;e++)for(let t=r.i0-1;t<=r.i1+1;t++){if(t<0||e<0||t>=c||e>=l)continue;let n=o+(t+.5)*u,p=s+(e+.5)*d;(a?n>i.x0-u*f&&n<i.x1+u*f&&p>i.z0-d*f&&p<i.z1+d*f:t>=r.i0&&t<=r.i1&&e>=r.j0&&e<=r.j1)&&(h[e*c+t]=0)}if(a)for(let e=0;e<l;e++)for(let t=0;t<c;t++){let n=o+(t+.5)*u,r=s+(e+.5)*d;n>i.x0&&n<i.x1&&r>i.z0&&r<i.z1&&(h[e*c+t]=0)}}return _}function Vm(e,t){let n=Math.min(e.x1,t.x1)-Math.max(e.x0,t.x0),r=Math.min(e.z1,t.z1)-Math.max(e.z0,t.z0);if(n<=0||r<=0)return e;let i={...e};return n<r?(e.x0+e.x1)/2>(t.x0+t.x1)/2?i.x0=t.x1:i.x1=t.x0:(e.z0+e.z1)/2>(t.z0+t.z1)/2?i.z0=t.z1:i.z1=t.z0,i}function Hm(e,t,n,r,i,a){let o=new Int32Array(t),s=null,c=0,l=[];for(let u=0;u<n;u++){for(let n=0;n<t;n++)o[n]=e[u*t+n]?o[n]+1:0;l.length=0;for(let e=0;e<=t;e++){let n=e<t?o[e]:0;for(;l.length&&o[l[l.length-1]]>=n;){let t=o[l.pop()],n=l.length?l[l.length-1]+1:0,d=(e-n)*r,f=t*i,p=d*f;t>0&&p>c&&d>=a&&f>=a&&(c=p,s={i0:n,i1:e-1,j0:u-t+1,j1:u})}l.push(e)}}return s}function Um(e,t,n,r,i){let a={...e},o=.005,s=(e,r)=>{if(!Lm(e,r,t))return!1;for(let t of n)if(Rm(e,r,t,o))return!1;return!0},c=(e,r)=>{let i=e.x0+(r===`x0`?0:o),a=e.x1-(r===`x1`?0:o),c=e.z0+(r===`z0`?0:o),l=e.z1-(r===`z1`?0:o);if(a<i){if(r===`x0`)a=i;else if(r===`x1`)i=a;else return!0}if(l<c){if(r===`z0`)l=c;else if(r===`z1`)c=l;else return!0}let u=Math.max(1,Math.ceil((a-i)/.35)),d=Math.max(1,Math.ceil((l-c)/.35));for(let e=0;e<=u;e++){let t=i+(a-i)*e/u;if(!s(t,c)||!s(t,l))return!1}for(let e=1;e<d;e++){let t=c+(l-c)*e/d;if(!s(i,t)||!s(a,t))return!1}for(let[n,r]of t)if(Rm(n,r,e,o))return!1;for(let t of n)if(e.x0<t.x1-o&&e.x1>t.x0+o&&e.z0<t.z1-o&&e.z1>t.z0+o)return!1;return!0},l=(e,t)=>e===`x0`?{...a,x0:a.x0-t,x1:a.x0}:e===`x1`?{...a,x0:a.x1,x1:a.x1+t}:e===`z0`?{...a,z0:a.z0-t,z1:a.z0}:{...a,z0:a.z1,z1:a.z1+t};for(let e=0;e<2;e++)for(let e of[`x0`,`x1`,`z0`,`z1`]){let t=e===`x0`||e===`z0`?-1:1,n=0,o=(e[0]===`x`?r:i)*1.5;if(c(l(e,o),e))n=o;else for(let t=0;t<7;t++){let t=(n+o)/2;c(l(e,t),e)?n=t:o=t}a[e]+=t*n}return a}const Wm={roofPitch:23,pitchVary:4,eaves:.5,minEave:2.5,forms:{hip:6,gable:2.5,dutch:1},squareHip:1.15,frontGable:.7,lean:{chance:.75,depth:5,pitch:8,low:2.2},chimneyChance:.35,windowSpacing:4,windowHeight:1.2,walls:{"#9a4a32":3,"#b8693f":2,"#d7c19a":2,"#ece7dc":2,"#cfc8b8":1},roofs:{"#a5502f":3,"#7a4a35":2,"#4a4d52":2,"#8f9399":1,"#5b3a2c":1},windowColor:`#273240`,doorColor:`#5b3b25`,soffit:`#6d6a64`},Gm={walls:[`#e8e2d4`,`#d9d0bf`,`#cdbfa8`,`#bfa58b`,`#c9c6bf`,`#b9b4ab`],roofs:[`#7b7d80`,`#6a6d70`,`#a4a7a9`,`#5e6266`],window:`#4a5a6c`},Km=/* @__PURE__ */ new Set([`house`,`residential`,`detached`,`semidetached_house`,`terrace`,`bungalow`,`farm`]),qm=(e,t,n)=>Km.has(e)||!e&&t<450&&n<10&&t>40,Jm=e=>bm(e)>0?[...e]:[...e].reverse(),Ym=e=>s(e.flat(),[],2);function Xm(e,t,n,r,i=Gm,a=!1,o=-1){let s=Jm(t),l=s.length,u=c(r.pick(i.walls),r.next(),.06),d=r.pick(i.roofs);if(e.prism(s,o,n,u,d,Ym(s)),!(n<=6||a)){e.material($e.windows);for(let t=0;t<l;t++){let[r,a]=s[t],[o,c]=s[(t+1)%l],u=Math.hypot(o-r,c-a);if(u<3)continue;let d=(o-r)/u,f=(c-a)/u,p=-f*.06,m=d*.06,h=Math.max(1,Math.floor(u/3.2)),g=Math.min(1.8,u/h-.8);for(let t=1.1;t+1.4<n-.6;t+=3.4)for(let n=0;n<h;n++){let o=(n+.5)/h*u;e.wall(r+d*(o-g/2)+p,a+f*(o-g/2)+m,r+d*(o+g/2)+p,a+f*(o+g/2)+m,t,t+1.4,i.window)}}e.material(0)}}function Zm(e,t,n,r,a=!1,o=-.3,s=!0){if(Jm(t.ring).length<3)return null;let l=Am(t.ring,t.front?[t.front.x,t.front.z]:null,n.plan),u=l.main,d=u.x1-u.x0,f=u.z1-u.z0;if(!(d>1&&f>1))return null;let p=(e,t,n)=>[e*l.x[0]+n*l.z[0],t,e*l.x[1]+n*l.z[1]],m=Math.min(n.eaves,Math.min(d,f)*.2),h=n.roofPitch+(r.next()*2-1)*n.pitchVary,g=Math.max(d,f)/Math.max(.1,Math.min(d,f)),_=Object.entries(n.forms).filter(([,e])=>(e??0)>0),v=_.length?r.weighted(Object.fromEntries(_)):`hip`,y=n.roof??null,b=y?`gable`:g<n.squareHip?`hip`:v,x=b===`skillion`?Math.min(h,n.lean.pitch):h,S=i(b,d,f,{pitch:x}).rise,C=Math.max(0,n.raise??0),w=t.height-C,T=w-S>4.6?2:1,E=Math.max(n.minEave,Math.min(T===2?6.2:3.2,w-S)),D=c(r.weighted(n.walls),r.next(),.06),O=c(r.weighted(n.roofs),r.next(),.06),k=c(n.soffit,r.next(),.05),A=(t,r,i,c,l)=>{let u=t.x0+r.x0,d=t.x1-r.x1,f=t.z0+r.z0,m=t.z1-r.z1;if(d-u<.5||m-f<.5)return;let h=[[`front`,u,m,d,m],[`right`,d,m,d,f],[`back`,d,f,u,f],[`left`,u,f,u,m]];for(let[t,r,u,d,f]of h){if(t===c)continue;let m=p(r,0,u),h=p(d,0,f);e.wall(m[0],m[2],h[0],h[2],C>0?C:o,C+i,D);let g=Math.hypot(h[0]-m[0],h[2]-m[2]);if(a||g<2.2)continue;let _=l&&t===`front`,v=Math.max(_?2:1,Math.floor(g/n.windowSpacing)),y=(h[0]-m[0])/g,b=(h[2]-m[2])/g,x=-b,S=y;for(let t=0;t<(i>4.6?2:1);t++)for(let r=0;r<v;r++){if(_&&r===0&&t===0)continue;let a=(r+.5)/v*g,o=Math.min(.75,g/v/3),c=.9+t*2.9,l=Math.min(i-.25,c+n.windowHeight),u=.03;l-c<.4||(s&&e.material($e.windows),e.wall(m[0]+y*(a-o)+x*u,m[2]+b*(a-o)+S*u,m[0]+y*(a+o)+x*u,m[2]+b*(a+o)+S*u,C+c,C+l,n.windowColor),s&&e.material(0))}if(_){let t=.5/v*g,r=Math.min(.45,g/v/2.5),a=.05;e.wall(m[0]+y*(t-r)+x*a,m[2]+b*(t-r)+S*a,m[0]+y*(t+r)+x*a,m[2]+b*(t+r)+S*a,C,C+Math.min(2.1,i-.2),n.doorColor),C>0&&j(m[0]+y*t,m[2]+b*t,x,S,r*2+.3)}}},j=(t,r,i,a,s)=>{let c=Math.max(1,Math.ceil(C/.18)),l=C/c,u=.27,d=Math.atan2(a,i)+Math.PI/2;for(let f=0;f<c;f++){let c=.05+u*(f+.5),p=C-l*f;e.box(t+i*c,r+a*c,o,s,p-o,u,d,n.posts?.floor??D)}},M=(t,r)=>{let i=t.x0+r.x0,s=t.x1-r.x1,c=t.z0+r.z0,u=t.z1-r.z1;if(s-i<.5||u-c<.5)return;let d=Math.atan2(l.x[1],l.x[0]),f=p((i+s)/2,0,(c+u)/2),m=.22,h=n.posts?.color??`#6b5b4b`;if(e.box(f[0],f[2],C-m,s-i,m,u-c,d,n.posts?.floor??D),a)return;let g=n.posts?.spacing??2.4,_=n.posts?.size??.22,v=Math.max(1,Math.round((s-i)/g)),y=Math.max(1,Math.round((u-c)/g));for(let t=0;t<=v;t++)for(let n=0;n<=y;n++){let r=p(i+_/2+(s-i-_)*t/v,0,c+_/2+(u-c-_)*n/y);e.box(r[0],r[2],o,_,C-m-o,_,d,h)}},N=(t,n,r,o)=>{let s=C+r;if(y&&n!==`skillion`){let n=t.x1-t.x0,r=t.z1-t.z0,c=i(`gable`,n,r,o).ridge,u=i(`gable`,n,r,o).rise,d=p((t.x0+t.x1)/2,s,(t.z0+t.z1)/2),f=c===`x`?l.x:l.z;return y.into(e,{width:c===`x`?n:r,depth:c===`x`?r:n,height:u},{at:d,turn:Math.atan2(f[1],f[0])*180/Math.PI,level:a?2:0}),u}let c=(t.x0+t.x1)/2,u=(t.z0+t.z1)/2,d=Bt(n,t.x1-t.x0,t.z1-t.z0,o),f=e=>p(c+e[0],s+e[1],u+e[2]);for(let t=0;t+2<d.slopes.length;t+=3)e.tri(f(d.slopes[t]),f(d.slopes[t+1]),f(d.slopes[t+2]),O);for(let t=0;t+2<d.ends.length;t+=3)e.tri(f(d.ends[t]),f(d.ends[t+1]),f(d.ends[t+2]),D);if(!a)for(let t=0;t+2<d.under.length;t+=3)e.tri(f(d.under[t]),f(d.under[t+1]),f(d.under[t+2]),k);return d.rise},P={x0:m,x1:m,z0:m,z1:m};C>0&&M(u,P),A(u,P,E,null,!0),N(u,b,E,{pitch:x,ridge:`long`,endInset:m});for(let e of l.wings){let t=e.x1-e.x0,i=e.z1-e.z0,a=Math.min(m,Math.min(t,i)*.2),o={front:`z0`,back:`z1`,left:`x1`,right:`x0`,apart:null}[e.side],s=o===`z0`||o===`z1`?i:t,c={...e};o&&(c[o]+=o===`x0`||o===`z0`?-m:m);let u=!!o&&Pm(l,c),p={x0:a,x1:a,z0:a,z1:a};o&&u&&(p[o]=-m);let g=u?Qm[e.side]:null;if(o&&e.side!==`front`&&s<=n.lean.depth&&r.chance(n.lean.chance)){let t=E-.15,r=Math.max(n.lean.low,t-s*Math.tan(n.lean.pitch*Math.PI/180));C>0&&M(e,p),A(e,p,r,g,!1),N(u?c:e,`skillion`,r,{rise:t-r,high:o===`z1`?`+z`:o===`z0`?`-z`:o===`x1`?`+x`:`-x`,endInset:a});continue}let _=o===`z0`||o===`z1`?`z`:o||t>=i?`x`:`z`,v=(_===`x`?i:t)/2,y=e.side===`front`&&r.chance(n.frontGable)||b===`dutch`&&v<2.5?`gable`:b,x=Math.min(h,Math.atan(S*.92/Math.max(.5,v))*180/Math.PI);C>0&&M(e,p),A(e,p,E,g,!1),N(o?Fm(l,e,o,Math.min(v+.3,(o[0]===`x`?d:f)/2)):e,y,E,{pitch:x,ridge:_,endInset:a})}if(!a&&r.chance(n.chimneyChance)){let[t,,n]=p((u.x0+u.x1)/2+r.range(-.25,.25)*d,0,(u.z0+u.z1)/2+r.range(-.15,.15)*f);e.box(t,n,C+E+S*.4,.6,S*.6+.9,.6,Math.atan2(l.x[1],l.x[0]),D,`#5a4a40`)}return{eave:C+E,top:C+E+S}}const Qm={front:`back`,back:`front`,left:`right`,right:`left`,apart:null},$m={fence:{chance:.85,types:{brick:3,picket:3,paling:2,none:1},brickHeight:.7,picketHeight:1.1,palingHeight:1.7,driveway:3.2,brickColor:`#a9583c`,picketColor:`#efede6`,palingColor:`#8a6b4f`},bins:{chance:.95,body:`#2f3a32`,general:`#c0392b`,recycling:`#f1c40f`,garden:`#27ae60`},driveway:{chance:.85,width:2.8,color:`#a6a299`,lift:.06,step:4}};function eh(e,t,n,r,i,a){if(!i||i.length<3)return;let{px:o,pz:s,tx:c,tz:l,nx:u,nz:d}=t,f=a.width/2,p=(e,t)=>[o+c*e+u*t,s+l*e+d*t],m=1/0,h=-1/0,g=r.length,_=r.map(([e,t])=>(e-o)*c+(t-s)*l),v=r.map(([e,t])=>(e-o)*u+(t-s)*d);for(let e=0;e<g;e++){h=Math.max(h,v[e]),_[e]>=n-f&&_[e]<=n+f&&(m=Math.min(m,v[e]));let t=(e+1)%g;for(let r of[n-f,n,n+f])(_[e]-r)*(_[t]-r)<0&&(m=Math.min(m,v[e]+(v[t]-v[e])*(r-_[e])/(_[t]-_[e])))}let y=Number.isFinite(m)?m-.3:h+1,b=e=>wm(...p(n-f,e),i)&&wm(...p(n+f,e),i);if(!b(.3))return;for(;y>2&&!b(y);)--y;if(y<=2)return;let x=Math.max(1,Math.ceil((y-.3)/a.step)),S=[];for(let e=0;e<=x;e++)S.push(p(n,.3+(y-.3)*e/x));e.ribbon(S,-f,f,a.lift,a.color)}function th(e,t,n,r,i){if(r===`none`)return;let a=Math.hypot(n[0]-t[0],n[1]-t[1]);if(a<.5)return;let o=r===`brick`?i.brickHeight:r===`picket`?i.picketHeight:i.palingHeight,s=r===`brick`?i.brickColor:r===`picket`?i.picketColor:i.palingColor,c=(n[0]-t[0])/a,l=(n[1]-t[1])/a,u=-l*.06,d=c*.06,f=Math.max(1,Math.ceil(a/3));for(let n=0;n<f;n++){let r=n/f*a,i=(n+1)/f*a,p=[t[0]+c*(r+i)/2,t[1]+l*(r+i)/2];e.on(p),e.wall(t[0]+c*r+u,t[1]+l*r+d,t[0]+c*i+u,t[1]+l*i+d,0,o,s),e.wall(t[0]+c*i-u,t[1]+l*i-d,t[0]+c*r-u,t[1]+l*r-d,0,o,s)}}function nh(e,t,n,r,i){let a=Math.atan2(r.z,r.x);[i.general,i.recycling,i.garden].forEach((o,s)=>{let c=(s-1)*.65,l=t+r.x*c,u=n+r.z*c;e.on([l,u]),e.box(l,u,0,.55,1,.6,a,i.body,o)})}function rh(e,t=1e3){return e.map(e=>{let n=atob(e),r=[],i=[0,0,0],a=0,o=1;for(let e=0;e<n.length;e++){let s=n.charCodeAt(e);if(a+=s%128*o,s>=128){if(o*=128,o>2**49)throw Error(`Invalid numeric varint`);continue}let c=r.length%3,l=a%2?-(a+1)/2:a/2;i[c]=i[c]+l,r.push(i[c]/t),a=0,o=1}if(o!==1)throw Error(`Truncated numeric varint`);return r})}function ih(e){if(e.v!==void 0&&e.v!==1&&e.v!==2&&e.v!==3)throw new K(`E-DATA-004`,`unsupported road chunk version '${e.v}' (expected 1, 2 or 3)`,{fix:`recompile the roads with a compatible version of aslnut`});if(e.numeric){if(e.pieces=rh(e.numeric,e.numericScale??1e3),e.turnUnits)for(let t of e.pieces)t[0]===3&&(t[9]/=e.turnUnits);delete e.numeric}return e}const ah=[`roads`,`buildings`,`parks`,`water`,`streams`,`lots`],oh=[`trees`,`crowns`,`anchors`,`pads`,`seeds`,`tseeds`,`fronts`,`furniture`,`homes`];function sh(e){let t=e.numeric;if(!t)return e;let n=e;for(let e of ah)n[e]=t[e]?rh(t[e],1).map(e=>e.map(Math.round)):[];for(let e of oh)n[e]=t[e]?rh(t[e],1)[0].map(Math.round):[];let r=(e,t)=>{let n=[];for(let r=0;r+t-1<e.length;r+=t)n.push(e.slice(r,r+t));return n};n.fronts=r(n.fronts,7);let i=n.trees,a=n.crowns,o=[];for(let e=0;e*3+2<i.length;e++)o.push(i[3*e],i[3*e+1],i[3*e+2],a[e]??0);return n.trees=o,delete n.crowns,n.patches=t.patches?r(rh(t.patches,1)[0].map(Math.round),6):[],delete n.numeric,e}const ch={water:`#3f7fae`,stream:`#3f7fae`,sea:`#2f6a96`};function lh(e,t,n=ch){e.material(Bn.water).floor(t,.25,n.water??ch.water,s(t.flat(),[],2)).material(0)}function uh(e,t,n,r=ch){e.material(Bn.water).ribbon(xm(t,6),-n/2,n/2,.25,r.stream??ch.stream).material(0)}function dh(e){let t=Number(e.width);if(Number.isFinite(t)&&t>0)return t;let n=String(e.waterway??e.feature_type_code??e.kind??``).toLowerCase();return/river/.test(n)?20:/canal/.test(n)?12:/channel|drain/.test(n)?3:/ditch/.test(n)?1.5:/stream|creek|watercourse/.test(n)?4:3}const fh=110574;var ph=class extends Zt{stands=!1;wp;srcs=[];offs=[];seenV=``;constructor(e){super({hover:!1,shadows:!1,...e}),this.wp=e}async attach(e){await super.attach(e);for(let t of[this.wp.lakes,this.wp.rivers])t instanceof Te&&(this.srcs.push(t),this.offs.push(t.watch(()=>this.build())),t.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.build()));this.build()}detach(){for(let e of this.offs)e();this.offs=[],this.srcs=[],super.detach()}feats(e){return e?(e instanceof Te?e.features():e).map(e=>e.props?e:{...e,props:e.properties??{}}):[]}build(){let e=this.srcs.map(e=>e.version).join(`,`);if(e===this.seenV&&this.seenV!==``)return;this.seenV=e||`none`;for(let e of this.keys())this.drop(e);let t=[];for(let e of this.feats(this.wp.lakes)){let n=e.geometry;if(n)for(let r of n.type===`Polygon`?[n.coordinates]:n.type===`MultiPolygon`?n.coordinates:[])t.push({f:e,lake:!0,pts:r[0]})}for(let e of this.feats(this.wp.rivers)){let n=e.geometry;if(n)for(let r of n.type===`LineString`?[n.coordinates]:n.type===`MultiLineString`?n.coordinates:[])t.push({f:e,lake:!1,pts:r})}if(!t.length)return;let n=t[0].pts[0],r=[n[0],n[1]],i=111320*Math.cos(r[1]*G),a=/* @__PURE__ */ new Map;for(let e of t){let t=e.pts[0],n=`${Math.floor((t[0]-r[0])*i/500)}_${Math.floor(-(t[1]-r[1])*fh/500)}`,o=a.get(n);o||a.set(n,o=[]),o.push(e)}for(let[e,t]of a){let[n,a]=e.split(`_`).map(Number),o=new ce,s=[];t.forEach((t,c)=>{let l=t.pts.map(([e,t])=>[(e-r[0])*i-n*500,-(t-r[1])*fh-a*500]);if(o.as(c),s[c]={id:t.f.id??`${e}:${c}`,props:{...t.f.props}},t.lake)l.length>3&&lh(o,l.slice(0,-1),this.wp.look);else{let e=this.wp.width?.(t.f)??dh(t.f.props);s[c].props[`width (m)`]=e,uh(o,l,e,this.wp.look)}}),this.put(e,{origin:r,side:500,at:[n*500,a*500],mesh:o.build(),info:e=>{let t=s[e];return t?{kind:`water`,id:t.id,props:t.props}:null}})}}seaLevel(e){let t=this.wp.level;return typeof t==`function`?t(e):t??0}frame(e){return this.far=e.camera.dist>(this.wp.near??8e3),this.far?[]:super.frame(e)}far=!1;animating(){return!this.far&&super.animating()}wantsFrames(e){return this.far?0:super.wantsFrames(e)}status(){let e=super.status();return this.far?{...e,pending:[],ready:!e.failed.length}:e}sea(e){return this.wp.sea===!1?null:{level:this.seaLevel(e),color:this.seaColour}}get seaColour(){let e=(this.wp.look?.sea??ch.sea).replace(`#`,``);return[parseInt(e.slice(0,2),16)/255,parseInt(e.slice(2,4),16)/255,parseInt(e.slice(4,6),16)/255]}};const mh=(e={})=>new ph(e),hh={spacing:42,height:10,chance:.75,wires:!0,color:`#6b5a46`,wire:`#1f2328`};function gh(e,t,n,r,i){let a=r.spacing*.5,o=null,s=0;for(let c=0;c+1<t.length;c++){let[l,u]=t[c],[d,f]=t[c+1],p=Math.hypot(d-l,f-u);if(p<.01)continue;let m=(d-l)/p,h=(f-u)/p,g=-h,_=m;for(let t=a;t<p;t+=r.spacing){let c=l+m*t+g*n,d=u+h*t+_*n,f=Math.atan2(h,m);if(e.on([c,d]),e.box(c,d,-.2,.28,r.height+.2,.28,f,r.color),e.box(c,d,r.height-.6,.12,.12,2,f,r.color),r.wires&&o)for(let e of[-.85,.85])i.push({ax:o[0]+g*e,az:o[1]+_*e,bx:c+g*e,bz:d+_*e,top:r.height-.5});o=[c,d],a=t,s++}a-=p,a<0&&(a+=r.spacing)}return s}var _h=class extends Lo{w;pf=/* @__PURE__ */ new Int32Array;constructor(e){super({width:1,color:hh.wire,pickable:!1,detail:{step:0xe8d4a51000},...e,on:`absolute`}),this.w=e}spanOf(e){if(this.w.span)return this.w.span(e);let t=e;return t.a?t:t.props}read(e){return super.read(e).map(e=>{let t=this.spanOf(e.d),n=[];for(let e=0;e<=8;e++)n.push([t.a[0]+(t.b[0]-t.a[0])*e/8,t.a[1]+(t.b[1]-t.a[1])*e/8]);return{...e,geometry:{type:`LineString`,coordinates:n}}})}built2(){let e=this.segs,t=new Int32Array(e.h.length).fill(-1);for(let n=0;n<e.count;n++){let r=n*5;t[e.seg[r]]=e.seg[r+4],t[e.seg[r+1]]=e.seg[r+4]}this.pf=t}ground(e){let t=this.segs,n=e.world.exaggeration,r=this.w.sag??.6,i=/* @__PURE__ */ new Map,a=t=>{let r=e.world.heightAt(t[0],t[1]);return Number.isNaN(r)?0:r*n};t.heights(e=>{let n=this.pf[e];if(n<0)return 0;let o=i.get(n);if(!o){let e=this.spanOf(this.bySlot[n].d);o=[a(e.a),a(e.b),e.top],i.set(n,o)}let s=t.frac[e];return o[0]+(o[1]-o[0])*s+o[2]-r*4*s*(1-s)}),this.upload(e)}};const vh=e=>new _h(e),$={carriageway:.12,kerb:.27,strip:.2,footpath:.24,platform:.13,markings:.02,skirt:.4};function yh(e,t=[]){let n=[];for(let t of e){let e=gm(t.pts),r=t.profile,i=_m(t.pts,t.trim[0],e-t.trim[1]);if(!(i.length<2))for(let e of[`left`,`right`]){let a=r[e];if(!(a.footpath>0))continue;let o=r.carriageway/2+a.kerb+a.strip+a.footpath/2;n.push({line:bh(i,e===`left`?-o:o),of:t.id,side:e,width:a.footpath})}}for(let e of t)for(let t of e.corners){let r=t.bands.find(e=>e.kind===`footpath`);r&&n.push({line:r.inner.map((e,t)=>[(e[0]+r.outer[t][0])/2,(e[1]+r.outer[t][1])/2]),of:e.key,side:`corner`,width:Math.hypot(r.outer[0][0]-r.inner[0][0],r.outer[0][1]-r.inner[0][1])})}return n}function bh(e,t){return e.map((n,r)=>{let i=e[Math.max(0,r-1)],a=e[Math.min(e.length-1,r+1)],o=Math.hypot(a[0]-i[0],a[1]-i[1])||1;return[n[0]-(a[1]-i[1])/o*t,n[1]+(a[0]-i[0])/o*t]})}const xh=[`Freeway`,`Highway`,`Arterial road`,`Sub-arterial road`,`Collector road`,`Local street`,`Track`,`Track`,`Lane`,`Lane`],Sh={kerb:.45,strip:3,footpath:1.4,arterialStrip:1.5,kerbColor:`#b7b4ab`,stripColor:`#6e9b46`,footpathColor:`#cfcbc1`,asphalt:[`#34373c`,`#3b3e43`,`#45484d`],track:`#8a8473`,marking:`#e8e6df`},Ch=(e,t)=>e<=1?t.asphalt[0]:e<=3?t.asphalt[1]:e<=5?t.asphalt[2]:t.track;function wh(e,t,n,r=Sh,i=!1){let a=xm(t,i?24:6),o=n.width/2,s=.3+(9-Math.min(9,n.cls))*.02;if(e.ribbon(a,-o,o,s,Ch(n.cls,r)),n.cls>4||i)return;let c=Math.max(2,Math.round(n.width/3.5)),l=n.width/c;for(let t=1;t<c;t++){let n=-o+t*l,i=0;for(let t=0;t+1<a.length;t++){let[o,c]=a[t],[l,u]=a[t+1],d=Math.hypot(l-o,u-c);for(let t=9-i%9;t<d;t+=9){let i=Math.min(d,t+3),a=t/d,f=i/d;t<0||e.ribbon([[o+(l-o)*a,c+(u-c)*a],[o+(l-o)*f,c+(u-c)*f]],n-.06,n+.06,s+.02,r.marking)}i+=d}}}function Th(e,t=Sh){if(e.cls<=1||e.cls>=6)return[];let n=e.width/2,r=e.cls===2?t.arterialStrip:t.strip;return[{from:n,to:n+t.kerb,color:t.kerbColor,lift:.2},{from:n+t.kerb,to:n+t.kerb+r,color:t.stripColor,lift:.12},{from:n+t.kerb+r,to:n+t.kerb+r+t.footpath,color:t.footpathColor,lift:.16}]}function Eh(e,t=Sh){let n=Th(e,t);return n.length?n[n.length-1].to:e.width/2+1}function Dh(e,t,n,r=Sh){let i=Th(n,r);if(!i.length)return;let a=xm(t,6);for(let t of i)e.ribbon(a,t.from,t.to,t.lift,t.color),e.ribbon(a,-t.to,-t.from,t.lift,t.color)}const Oh={...Sh,crossing:`#f2f1ea`,giveWay:`#f2f1ea`,gapColor:`#7aa553`},kh=(e,t)=>e===`kerb`?t.kerbColor:e===`strip`?t.stripColor:t.footpathColor,Ah=e=>e===`kerb`?$.kerb:e===`strip`?$.strip:$.footpath,jh=(e,t,n,r)=>{t.length>=3&&e.floor(t,n,r,s(t.flat(),[],2))},Mh=(e,t,n,r,i)=>{for(let a=0;a+1<t.length;a++)e.wall(t[a][0],t[a][1],t[a+1][0],t[a+1][1],n,r,i)};function Nh(e,t,n=Oh,r=!1){let i=gm(t.pts),a=xm(_m(t.pts,t.trim[0],i-t.trim[1]),6),o=t.profile,s=o.carriageway/2,c=Ch(t.cls,n),l=o.median/2;if(l>0&&!r?(e.ribbon(a,-s,-l,$.carriageway,c),e.ribbon(a,l,s,$.carriageway,c),e.ribbon(a,-l,-l+.45,$.kerb,n.kerbColor),e.ribbon(a,l-.45,l,$.kerb,n.kerbColor),e.ribbon(a,-l+.45,l-.45,$.strip,n.stripColor),Mh(e,bh(a,-l),$.carriageway-.05,$.kerb,n.kerbColor),Mh(e,bh(a,l),$.carriageway-.05,$.kerb,n.kerbColor)):e.ribbon(a,-s,s,$.carriageway,c),!r){t.cls<=4&&!(l>0)&&Ph(e,a,t.cls,o.carriageway,n);for(let t of[`left`,`right`]){let r=o[t],i=t===`left`?-1:1,c=s;r.kerb>0&&Mh(e,bh(a,i*s),$.carriageway-.05,$.kerb,n.kerbColor);for(let[t,o]of[[`kerb`,r.kerb],[`strip`,r.strip],[`footpath`,r.footpath]]){if(!(o>.05))continue;let r=c*i,s=(c+o)*i;e.ribbon(a,Math.min(r,s),Math.max(r,s),Ah(t),kh(t,n)),c+=o}if(r.gap>.05){let t=c*i,o=(c+r.gap)*i;e.ribbon(a,Math.min(t,o),Math.max(t,o),$.strip,n.gapColor),c+=r.gap}c>s&&Mh(e,bh(a,i*c),-$.skirt,$.strip,n.gapColor)}}}function Ph(e,t,n,r,i){let a=Math.max(2,Math.round(r/3.5)),o=r/a,s=r/2;for(let n=1;n<a;n++){let r=-s+n*o,a=0;for(let n=0;n+1<t.length;n++){let[o,s]=t[n],[c,l]=t[n+1],u=Math.hypot(c-o,l-s);for(let t=12-a%12;t<u;t+=12){let n=Math.min(u,t+3),a=t/u,d=n/u;e.ribbon([[o+(c-o)*a,s+(l-s)*a],[o+(c-o)*d,s+(l-s)*d]],r-.06,r+.06,$.carriageway+$.markings,i.marking)}a+=u}}}function Fh(e,t,n=Oh,r=!1){let i=Math.min(...t.arms.map(e=>e.cls));{let r=[t.at,...t.platform],a=t.platform.length,o=[];for(let e=0;e<a;e++)o.push(0,1+e,1+(e+1)%a);e.floor(r,$.platform,Ch(i,n),o)}if(!r){for(let r of t.corners){r.bands.length&&Mh(e,r.curve,$.platform-.05,$.kerb,n.kerbColor);for(let t of r.bands){let r=[...t.inner,...[...t.outer].reverse()];jh(e,r,Ah(t.kind),kh(t.kind,n))}let t=r.bands[r.bands.length-1];t&&Mh(e,t.outer,-$.skirt,Ah(t.kind),kh(t.kind,n))}for(let r of t.crossings){let t=r.b[0]-r.a[0],i=r.b[1]-r.a[1],a=Math.hypot(t,i),o=t/a,s=i/a,c=Math.floor(a/1.2);for(let t=0;t<c;t++){let i=.3+t*1.2,c=i+.6;if(c>a-.2)break;let l=[r.a[0]+o*i,r.a[1]+s*i],u=[r.a[0]+o*c,r.a[1]+s*c];jh(e,[l,u,[u[0]+r.dir[0]*r.width,u[1]+r.dir[1]*r.width],[l[0]+r.dir[0]*r.width,l[1]+r.dir[1]*r.width]],$.platform+$.markings+.02,n.crossing)}}for(let r of t.giveWay){let t=r.b[0]-r.a[0],i=r.b[1]-r.a[1],a=Math.hypot(t,i),o=Math.max(1,Math.floor(a/1.2));for(let s=0;s<o;s++){let o=s*1.2/a,c=o+.6/a;e.ribbon([[r.a[0]+t*o,r.a[1]+i*o],[r.a[0]+t*c,r.a[1]+i*c]],-.15,.15,$.carriageway+$.markings+.02,n.giveWay)}}}}const Ih=(e,t)=>[e[0]-t[0],e[1]-t[1]],Lh=e=>{let t=Math.hypot(e[0],e[1])||1;return[e[0]/t,e[1]/t]},Rh=e=>[e[1],-e[0]],zh=e=>`${Math.round(e[0]*5)},${Math.round(e[1]*5)}`;function Bh(e,t={}){let n=t.radius??(e=>e<=3?15:e===4?12.5:7.5),r=t.crossings??(e=>e<=4),i=/* @__PURE__ */ new Map;for(let t of e)for(let e of[0,1]){let n=t.pts[e===0?0:t.pts.length-1],r=zh(n),a=i.get(r);a||i.set(r,a={p:[n[0],n[1]],ends:[]}),a.ends.push({s:t,end:e})}let a=[];for(let[e,t]of i){if(t.ends.length<2)continue;let i=t.p,o=t.ends.map(({s:e,end:t})=>{let n=gm(e.pts),r=hm(e.pts,t===0?Math.min(n,6):Math.max(0,n-6)),a=Lh(Ih([r.x,r.z],i));return{s:e,end:t,d:a,angle:Math.atan2(a[1],a[0]),L:n}}).sort((e,t)=>e.angle-t.angle);if(o.length===2){let e=Math.abs(o[1].angle-o[0].angle);if(e>Math.PI&&(e=2*Math.PI-e),e>Math.PI*.8)continue}let s=o.map(({s:e,end:t,d:n,angle:r})=>{let a=e.profile,o=t===0,s=o?a.left:a.right,c=o?a.right:a.left;return{section:e,end:t,at:i,dir:n,angle:r,half:a.carriageway/2,cls:e.cls,before:c,after:s}}),c=Math.max(...s.map(e=>e.half+1));for(let e=0;e<s.length;e++){let t=s[e],r=s[(e+1)%s.length],i=r.angle-t.angle;i<=0&&(i+=Math.PI*2);let a=Math.min(n(t.cls),n(r.cls));i<Math.PI*.95&&(c=Math.max(c,(t.half+r.half+.5)/Math.max(.2,Math.sin(Math.min(i,Math.PI/2)))+a*.4))}let l=Math.min(...o.map(e=>e.L*.45)),u=Math.min(c,l);for(let e of s)e.section.trim[e.end]=Math.max(e.section.trim[e.end],u);let d=(e,t)=>{let n=Rh(e.dir);return[i[0]+e.dir[0]*u+n[0]*e.half*t,i[1]+e.dir[1]*u+n[1]*e.half*t]},f=[],p=[],m=[],h=[],g=Math.min(...s.map(e=>e.cls));for(let e=0;e<s.length;e++){let t=s[e],n=s[(e+1)%s.length],r=d(t,-1),a=d(n,1);f.push(d(t,1),r);let o=n.angle-t.angle;o<=0&&(o+=Math.PI*2);let c=Vh(r,t.dir,a,n.dir,o,u*1.5+4);for(let e=1;e<c.length-1;e++)f.push(c[e]);let l=t.before,m=n.after,h=[],g=0;for(let e of[`kerb`,`strip`,`footpath`]){let t=Math.min(l[e],m[e])||Math.max(l[e],m[e])*0;t>.05&&(h.push({kind:e,inner:Hh(c,i,g),outer:Hh(c,i,g+t)}),g+=t)}p.push({curve:c,bands:h,from:e,to:(e+1)%s.length})}s.forEach((e,t)=>{if(r(e.cls)||r(g)&&e.cls>g){let n=Rh(e.dir),r=u+1.6;m.push({arm:t,a:[i[0]+e.dir[0]*r-n[0]*e.half,i[1]+e.dir[1]*r-n[1]*e.half],b:[i[0]+e.dir[0]*r+n[0]*e.half,i[1]+e.dir[1]*r+n[1]*e.half],dir:e.dir,width:3})}if(e.cls>g){let t=Rh(e.dir),n=u+(r(e.cls)||r(g)?3.6:.4);h.push({a:[i[0]+e.dir[0]*n,i[1]+e.dir[1]*n],b:[i[0]+e.dir[0]*n+t[0]*e.half,i[1]+e.dir[1]*n+t[1]*e.half]})}}),a.push({key:e,at:i,arms:s,trim:u,platform:f,corners:p,crossings:m,giveWay:h})}return a}function Vh(e,t,n,r,i,a=30){let o=[],s=null;if(i<Math.PI*.95){let i=t[0]*-r[1]-t[1]*-r[0];if(Math.abs(i)>1e-6){let o=n[0]-e[0],c=n[1]-e[1],l=(o*-r[1]-c*-r[0])/i,u=(t[0]*c-t[1]*o)/i;l<0&&u<0&&-l<a&&-u<a&&(s=[e[0]+t[0]*l,e[1]+t[1]*l])}}for(let t=0;t<=10;t++){let r=t/10,i=1-r;o.push(s?[i*i*e[0]+2*i*r*s[0]+r*r*n[0],i*i*e[1]+2*i*r*s[1]+r*r*n[1]]:[e[0]+(n[0]-e[0])*r,e[1]+(n[1]-e[1])*r])}return o}function Hh(e,t,n){return n?e.map((r,i)=>{let a=e[Math.max(0,i-1)],o=e[Math.min(e.length-1,i+1)],s=Lh([o[1]-a[1],-(o[0]-a[0])]);return s[0]*(r[0]-t[0])+s[1]*(r[1]-t[1])<0&&(s=[-s[0],-s[1]]),[r[0]+s[0]*n,r[1]+s[1]*n]}):e.map(e=>[e[0],e[1]])}const Uh={tree:{about:`a plain round-crowned broadleaf`,genera:[],spec:{trunk:{height:9,radius:.3,clear:.3},crown:`round`,levels:[{count:7,angle:55,length:.55,taper:.55},{count:4,angle:40,length:.5,taper:.6}],leaves:{kind:`broad`,size:2.8,density:2.6,colors:[`#4f7a3a`,`#5b8a3f`,`#466f35`]}}},plane:{about:`a London plane: tall, a broad round crown on a clear trunk, mottled bark`,genera:[`platanus`],spec:{trunk:{height:16,radius:.42,clear:.32},crown:`round`,levels:[{count:7,angle:50,length:.55,taper:.55},{count:4,angle:40,length:.5,taper:.6}],leaves:{kind:`broad`,size:3.4,density:3,colors:[`#5a8a3c`,`#679641`,`#4f7c36`]},bark:`#a59a80`}},elm:{about:`an elm: a vase of upswept limbs, a wide arching crown`,genera:[`ulmus`,`zelkova`],spec:{trunk:{height:15,radius:.4,clear:.28},crown:`vase`,levels:[{count:6,angle:32,length:.6,taper:.6,bend:18},{count:4,angle:45,length:.5,taper:.6}],leaves:{kind:`broad`,size:3,density:2.8,colors:[`#4c7a35`,`#56853c`,`#43702f`]},bark:`#5e5246`}},oak:{about:`an oak: a short stout trunk, a wide spreading crown`,genera:[`quercus`],spec:{trunk:{height:13,radius:.5,clear:.25},crown:`round`,levels:[{count:6,angle:65,length:.65,taper:.6,bend:4},{count:4,angle:45,length:.5,taper:.6}],leaves:{kind:`broad`,size:3.2,density:3.6,colors:[`#3f6a2e`,`#4a7634`,`#365f29`]},bark:`#4f4438`}},maple:{about:`a maple: a dense oval crown`,genera:[`acer`,`liquidambar`],spec:{trunk:{height:11,radius:.32,clear:.25},crown:`round`,levels:[{count:8,angle:45,length:.5,taper:.55},{count:4,angle:40,length:.5,taper:.6}],leaves:{kind:`broad`,size:2.8,density:2.9,colors:[`#5b8a3a`,`#6a9441`,`#4f7d33`]},bark:`#6a5c4c`}},ash:{about:`an ash: a light open crown of compound leaves`,genera:[`fraxinus`,`pistacia`],spec:{trunk:{height:12,radius:.32,clear:.33},crown:`round`,levels:[{count:6,angle:45,length:.55,taper:.55},{count:4,angle:38,length:.5,taper:.6}],leaves:{kind:`broad`,size:2.6,density:2.6,colors:[`#5f8c40`,`#6c9848`,`#557f39`]},bark:`#7a7166`}},eucalypt:{about:`a eucalypt (a gum): a pale leaning trunk, an open crown of narrow hanging leaves`,genera:[`eucalyptus`,`corymbia`,`angophora`],spec:{trunk:{height:18,radius:.42,clear:.45,lean:6},crown:`vase`,levels:[{count:5,angle:35,length:.5,taper:.5},{count:4,angle:45,length:.45,taper:.6}],leaves:{kind:`narrow`,size:3.1,density:3.3,colors:[`#6f8a5a`,`#7d9468`,`#5f7a4e`]},bark:`#cfc6b4`}},paperbark:{about:`a paperbark: a twisting pale trunk, a dense narrow-leaved crown`,genera:[`melaleuca`,`callistemon`,`lophostemon`,`tristaniopsis`],spec:{trunk:{height:8,radius:.3,clear:.3,lean:9},crown:`round`,levels:[{count:6,angle:50,length:.5,taper:.55,bend:6},{count:4,angle:45,length:.45,taper:.6}],leaves:{kind:`narrow`,size:2.4,density:3.8,colors:[`#5a7a4a`,`#668656`,`#4f6f41`]},bark:`#d9d1c2`}},wattle:{about:`a wattle: small, a light feathery crown`,genera:[`acacia`],spec:{trunk:{height:6,radius:.18,clear:.3,lean:5},crown:`round`,levels:[{count:6,angle:55,length:.55,taper:.55},{count:3,angle:45,length:.5,taper:.6}],leaves:{kind:`narrow`,size:1.8,density:3.4,colors:[`#6b8452`,`#76905c`,`#5f7748`]},bark:`#4d463d`}},lillypilly:{about:`a lilly pilly: a dense glossy rounded crown, low`,genera:[`syzygium`,`acmena`,`waterhousea`,`tristania`],spec:{trunk:{height:7,radius:.22,clear:.15},crown:`round`,levels:[{count:9,angle:50,length:.5,taper:.55},{count:4,angle:45,length:.45,taper:.6}],leaves:{kind:`broad`,size:2.1,density:3.1,colors:[`#2f5e2c`,`#386a33`,`#285324`]},bark:`#5a4c3e`}},blossom:{about:`a flowering plum or pear: small, a round crown of upswept branches`,genera:[`prunus`,`pyrus`,`malus`,`cercis`,`lagerstroemia`],spec:{trunk:{height:6.5,radius:.2,clear:.3},crown:`vase`,levels:[{count:6,angle:38,length:.55,taper:.55,bend:14},{count:4,angle:40,length:.5,taper:.6}],leaves:{kind:`broad`,size:1.9,density:3.6,colors:[`#6a8c45`,`#7b9a4f`,`#8c4a5a`]},bark:`#4a3b33`}},jacaranda:{about:`a jacaranda: a wide umbrella of fine leaves`,genera:[`jacaranda`,`gleditsia`,`robinia`,`albizia`],spec:{trunk:{height:10,radius:.28,clear:.4,lean:4},crown:`vase`,levels:[{count:5,angle:55,length:.7,taper:.55,bend:8},{count:4,angle:50,length:.5,taper:.6}],leaves:{kind:`narrow`,size:2.6,density:2.8,colors:[`#5e8a43`,`#6c9a4c`,`#7a6aa8`]},bark:`#6d6152`}},birch:{about:`a birch: a slender white trunk, a narrow light crown`,genera:[`betula`,`alnus`,`carpinus`],spec:{trunk:{height:12,radius:.2,clear:.3},crown:`conical`,levels:[{count:8,angle:50,length:.4,taper:.5,bend:-6},{count:3,angle:40,length:.5,taper:.6}],leaves:{kind:`broad`,size:2,density:3,colors:[`#6b9446`,`#77a04e`,`#5f873e`]},bark:`#e6e2d8`}},poplar:{about:`a Lombardy poplar: a tall narrow column`,genera:[`populus`],spec:{trunk:{height:18,radius:.32,clear:.08},crown:`columnar`,levels:[{count:12,angle:22,length:.3,taper:.5,bend:14},{count:2,angle:28,length:.5,taper:.6}],leaves:{kind:`broad`,size:2.3,density:2.2,colors:[`#4b7a38`,`#56853f`,`#426d31`]},bark:`#6b6255`}},cypress:{about:`a cypress: a dark dense column of needles`,genera:[`cupressus`,`juniperus`,`thuja`,`chamaecyparis`,`callitris`],spec:{trunk:{height:12,radius:.28,clear:.05},crown:`columnar`,levels:[{count:12,angle:25,length:.3,taper:.5},{count:2,angle:30,length:.5,taper:.6}],leaves:{kind:`needle`,size:2.2,density:5,colors:[`#2f4f2a`,`#355a2f`,`#284524`]},bark:`#5d4a3a`}},pine:{about:`a pine: a cone of tiered branches and needles`,genera:[`pinus`,`picea`,`abies`,`cedrus`,`araucaria`,`agathis`],spec:{trunk:{height:16,radius:.36,clear:.2},crown:`conical`,levels:[{count:14,angle:80,length:.3,taper:.45,bend:-4},{count:3,angle:45,length:.45,taper:.6}],leaves:{kind:`needle`,size:2.6,density:4.6,colors:[`#2e5130`,`#365c37`,`#284829`]},bark:`#5a4636`}},palm:{about:`a palm: a tall bare trunk, a head of long drooping fronds`,genera:[`phoenix`,`washingtonia`,`livistona`,`archontophoenix`,`syagrus`,`trachycarpus`,`cocos`,`roystonea`],spec:{trunk:{height:12,radius:.3,clear:.88},crown:`round`,levels:[{count:14,angle:72,length:.36,taper:.4,start:.9,bend:-45}],leaves:{kind:`narrow`,size:1.8,density:6,colors:[`#5c7a3c`,`#688644`,`#516d35`]},bark:`#7d6d58`}}},Wh=/* @__PURE__ */ new Map;for(let[e,t]of Object.entries(Uh))for(let n of t.genera)Wh.set(n,e);function Gh(e,t){if(typeof e!=`string`||!e.trim())return null;let n=e.trim().toLowerCase();if(n in Uh)return n;let r=n.split(/[\s×.]+/)[0];return t?.[r]??Wh.get(r)??null}const Kh={species:{tree:{spec:Uh.tree.spec}}},qh=/* @__PURE__ */ new WeakMap,Jh=(e,t)=>{let n=La({...e,seed:t});return{levels:[n.levels[0],n.levels[1],Qt(n.levels[0],{views:8})],height:n.height,crownRadius:n.crownRadius,crownBase:n.crownBase,transmissivity:n.transmissivity}};function Yh(e=Kh){let t=qh.get(e);if(!t){t=/* @__PURE__ */ new Map,qh.set(e,t);for(let[n,r]of Object.entries(e.species))for(let i=0;i<(e.variants??2);i++)t.set(`${n}:${i}`,Jh(r.spec,11+i*7))}return t}function Xh(e,t=Kh){let n=Yh(t),r=/* @__PURE__ */ new Map;for(let n of e){let e=new Tm(Em(n.seed,31)),{s:i,key:a,tree:o}=tg(n,e,t),s=Math.max(2,n.height),l=s/o.height,u=Qh(n,s,l,o),d=c(`#ffffff`,e.next(),.1),f=e.next()*Math.PI*2,p=1-$h(t,n,i,o.transmissivity),m=r.get(a);m||r.set(a,m=[]),m.push(n.x,n.z,0,n.id,u,l,u,f,d,p)}return[...r].map(([e,t])=>({shape:e,data:Float32Array.from(t),tree:n.get(e)}))}function Zh(e,t=Kh){let{s:n,tree:r}=tg(e,new Tm(Em(e.seed,31)),t),i=Math.max(2,e.height),a=i/r.height,o=Qh(e,i,a,r),s=eg(t,e,n,r.transmissivity)??r.transmissivity;return{top:i,base:r.crownBase*a,radius:r.crownRadius*o,transmissivity:Math.max(s,r.transmissivity)}}function Qh(e,t,n,r){if(!(e.radius>0))return n;let i=Math.max(1,Math.min(t*.6,e.radius));return Math.max(.6,Math.min(1.6,i/(r.crownRadius*n)))*n}function $h(e,t,n,r){let i=eg(e,t,n,r);return i===null||!(r>0&&r<1)?1:Math.max(0,Math.min(1,Math.log(i)/Math.log(r)))}function eg(e,t,n,r){let i=e.canopy;if(!i)return null;let a=i.transmit?i.transmit(t,n,r):i.density?Math.exp(-i.density(t,n,r)/2):null;return a===null?null:Math.max(.001,Math.min(1,a))}function tg(e,t,n){let r=Yh(n),i={};for(let[e,t]of Object.entries(n.species))i[e]=t.weight??1;let a=t.weighted(i),o=Math.floor(t.next()*(n.variants??2));if(e.species&&typeof e.species==`object`){let t=`custom:${JSON.stringify(e.species)}`,n=r.get(t);return n||(n=Jh(e.species,11),r.set(t,n)),{s:`custom`,key:t,tree:n}}if(e.species&&!n.species[e.species]){let t=Gh(e.species);if(t){let e=`library:${t}:${o}`,n=r.get(e);return n||(n=Jh(Uh[t].spec,11+o*7),r.set(e,n)),{s:t,key:e,tree:n}}}let s=e.species&&n.species[e.species]?e.species:a,c=`${s}:${o}`;return{s,key:c,tree:r.get(c)}}const ng=110574;var rg=class extends Zt{vp;src=null;seen=-1;off=null;infos=/* @__PURE__ */ new Map;constructor(e){super({hover:!1,...e}),this.vp=e}async attach(e){await super.attach(e);let t=this.vp.data;t instanceof Te?(this.src=t,this.off=t.watch(()=>this.build()),t.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.build())):this.build()}detach(){this.off?.(),this.off=null,super.detach()}build(){let e=(this.src?this.src.features():this.vp.data).map(e=>e.props?e:{...e,props:e.properties??{}});if(this.src&&this.src.version===this.seen)return;this.seen=this.src?.version??0;let t={...Kh,...this.vp.look},n=[],r=e=>this.vp.height?.(e)??Number(e.props.height??8),i=(e,t)=>{let n=this.vp.crown?this.vp.crown(e):Number(e.props.crown??t*.4);return n===null?0:n},a=0;for(let t of e){let e=t.geometry;if(!e)continue;let o=this.vp.species?.(t);if(e.type===`Point`){let[s,c]=e.coordinates,l=r(t);n.push({lon:s,lat:c,f:t,x:0,z:0,height:l,radius:i(t,l),id:a,seed:Em(s*1e5,c*1e5),species:o}),a++;continue}let s=this.vp.density?.(t)??0;if(!(s>0)||e.type!==`Polygon`&&e.type!==`MultiPolygon`)continue;let c=e.type===`Polygon`?[e.coordinates]:e.coordinates;for(let e of c){let c=e[0],l=1/0,u=-1/0,d=1/0,f=-1/0;for(let[e,t]of c)l=Math.min(l,e),u=Math.max(u,e),d=Math.min(d,t),f=Math.max(f,t);let p=111320*Math.cos((d+f)/2*G),m=(u-l)*p*(f-d)*ng/1e4,h=new Tm(Em(l*1e5,d*1e5,a));for(let e=0,p=Math.round(m*s);e<p*3&&n.length<2e5;e++){let e=h.range(l,u),s=h.range(d,f),p=!1;for(let t=0,n=c.length-1;t<c.length;n=t++){let[r,i]=c[t],[a,o]=c[n];i>s!=o>s&&e<(a-r)*(s-i)/(o-i)+r&&(p=!p)}if(!p)continue;let m=r(t)*h.range(.75,1.15);n.push({lon:e,lat:s,f:t,x:0,z:0,height:m,radius:i(t,m),id:a,seed:Em(e*1e5,s*1e5),species:o}),a++}}}for(let e of this.keys())this.drop(e);if(this.infos.clear(),!n.length)return;let o=[n[0].lon,n[0].lat],s=111320*Math.cos(o[1]*G),c=/* @__PURE__ */ new Map;for(let e of n){let t=(e.lon-o[0])*s,n=-(e.lat-o[1])*ng,r=Math.floor(t/500),i=Math.floor(n/500),a=`${r}_${i}`;e.x=t-r*500,e.z=n-i*500;let l=c.get(a);l||c.set(a,l=[]),l.push(e)}for(let[e,n]of c){let[r,i]=e.split(`_`).map(Number),a=/* @__PURE__ */ new Map;n.forEach((t,n)=>{t.id=n,a.set(n,{id:t.f.id??`${e}:${n}`,props:{...t.f.props,"height (m)":+t.height.toFixed(1)}})}),this.infos.set(e,a),this.put(e,{origin:o,at:[r*500,i*500],instances:Xh(n,t).map(e=>({shape:e.tree.levels,data:e.data,radius:e.tree.crownRadius*.8})),info:e=>{let t=a.get(e);return t?{kind:`tree`,id:t.id,props:t.props}:null}})}}};const ig=e=>new rg(e),ag={street:Sh,house:Wm,block:Gm,lot:$m,trees:Kh,poles:hh,water:ch,park:`#6f9a4c`,sports:`#5f9a45`},og=(e,t)=>{let n=[],r=e[t],i=e[t+1];n.push([r/10,i/10]);for(let a=t+2;a+1<e.length;a+=2)r+=e[a],i+=e[a+1],n.push([r/10,i/10]);return n},sg=[`measured`,`from floors`,`default`];function cg(e,t,n=ag,r={},i=()=>!0){sh(e);let a=new ce,o=new ce,c=[],l=t=>e.s?.[t]??``,u=/* @__PURE__ */ new Map;for(let[t,n,r,i,a,o]of e.patches??[]){let e=`${t}:${n}`,s=u.get(e)??[];s.push(`${l(r)}${o!==void 0&&o>=0?` · source row bld-${i}-${a}-${o}`:``}`),u.set(e,s)}let d=(e,t)=>{let n=u.get(`${e}:${t}`);return n?{placement:n.join(` · `)}:{}},f=e=>r[e]?`${r[e].title} (${r[e].licence})`:e,p=e=>(c.push(e),c.length-1),m=(e,n)=>t.groundAt(e,n),h={roads:[],houses:[],parks:[],spans:[]},g=t=>new Tm(e.seeds?.[t]??Em(e.ix,e.iz,t));for(let t of i(`park`)?e.parks:[]){let r=og(t,3),i=l(t[0]),u=l(t[1]);if(r.length<3)continue;let d=p({kind:`park`,id:`park-${e.ix}-${e.iz}-${c.length}`,props:{name:i||`open space`,group:u,"area (ha)":+(t[2]/1e4).toFixed(2),source:f(`parks`)}}),m=s(r.flat(),[],2),g=/organised|sport/i.test(u)?n.sports:n.park;a.as(d).floor(r,.08,g,m),o.as(d).floor(r,.08,g,m),h.parks.push({name:i,group:u,ring:r,id:d})}for(let t of i(`water`)?e.water:[]){let r=og(t,3);r.length<3||(a.as(p({kind:`water`,id:`water-${e.ix}-${e.iz}-${c.length}`,props:{name:l(t[0])||`water`,type:l(t[1]).replace(/_/g,` `),"area (ha)":+(t[2]/1e4).toFixed(2),source:f(`water`)}})),lh(a,r,n.water),o.as(a.idNow),lh(o,r,n.water))}for(let t of i(`waterway`)?e.streams:[]){let r=og(t,3);r.length<2||(a.as(p({kind:`waterway`,id:`stream-${e.ix}-${e.iz}-${c.length}`,props:{name:l(t[1])||`waterway`,type:l(t[2]).replace(/_/g,` `),"width (m)":t[0]/10,source:f(`water`)}})),uh(a,r,t[0]/10,n.water),o.as(a.idNow),uh(o,r,t[0]/10,n.water))}let _=(e.v??1)>=2,v=n.rules??vm,y=e.roads.map(e=>({cls:e[0],width:e[1]/10,name:l(e[2]),uid:_?l(e[3]):``,pts:og(e,_?4:3)})).filter(e=>e.pts.length>1).sort((e,t)=>t.cls-e.cls),b=(e.reserves??[]).filter(e=>e[0]===0).map(e=>og(e,1)),x=(e,t)=>ym(e,{reserve:b.length?dm(t,b):void 0},v),S=y.map((t,n)=>({id:t.uid||`road-${e.ix}-${e.iz}-${n}`,cls:t.cls,name:t.name,pts:t.pts,trim:[0,0],ends:[null,null],profile:x(t.cls,t.pts)})),C=[],w=i(`road`)&&!e.baked;if(w){if(_&&e.junctions){let t=new Map(S.map(e=>[e.id,e]));for(let n of e.junctions){let r=1,i=[];for(let e=0;e<n[0];e++){let e=n[r],a=l(n[r+1]),o=n[r+2],s=n[r+3],c=[],u=n[r+4],d=n[r+5];c.push([u/10,d/10]);for(let e=1;e<s;e++)u+=n[r+4+2*e],d+=n[r+5+2*e],c.push([u/10,d/10]);r+=4+2*s,i.push({id:`${a}@${o}`,cls:e,name:t.get(a)?.name??``,pts:c,trim:[0,0],ends:[null,null],profile:x(e,c)})}let[a]=Bh(i);if(a){for(let e of a.arms){let[n,r]=e.section.id.split(`@`),i=t.get(n);i&&(i.trim[Number(r)]=Math.max(i.trim[Number(r)],a.trim))}a.at[0]>=0&&a.at[1]>=0&&a.at[0]<e.size&&a.at[1]<e.size&&C.push(a)}}}else C=Bh(S)}let T=new Dm;y.forEach((t,r)=>{if(h.roads.push(t),!w)return;let i=S[r],s=i.profile,c=p({kind:`road`,id:i.id,props:{name:t.name||`unnamed road`,class:xh[t.cls]??`class ${t.cls}`,"carriageway (m)":+s.carriageway.toFixed(1),"footpath (m)":+Math.max(s.left.footpath,s.right.footpath).toFixed(1),"reserve (m)":s.reserve.left!==null&&s.reserve.right!==null?+(s.reserve.left+s.reserve.right).toFixed(1):`—`,"profile from":s.from===`reserve`?`the road reserve (Vicmap Property)`:s.from===`tags`?`tags`:`the class`,source:f(`roads`)}});a.as(c).on(null),o.as(c).on(null),Nh(a,i,{...Oh,...n.street}),Nh(o,i,{...Oh,...n.street},!0);let l=gm(i.pts),u=_m(i.pts,0,l),d=s.carriageway/2;if(T.addBand(u,-(d+s.left.kerb),d+s.right.kerb),s.left.footpath>0){let e=d+s.left.kerb+s.left.strip;T.addBand(u,-(e+s.left.footpath),-e)}if(s.right.footpath>0){let e=d+s.right.kerb+s.right.strip;T.addBand(u,e,e+s.right.footpath)}let m=g(7e3+r);if(t.cls>=4&&t.cls<=5&&n.poles.chance>0&&m.chance(n.poles.chance)){let i=m.chance(.5)?1:-1,o=i>0?s.right:s.left,c=(d+mm(o)-o.gap-o.footpath-.4)*i,l=p({kind:`pole`,id:`poles-${e.ix}-${e.iz}-${r}`,props:{what:`power poles (procedural)`,along:t.name||`unnamed road`,"height (m)":n.poles.height}});a.as(l),gh(a,t.pts,c,n.poles,h.spans)}}),C.forEach((t,r)=>{let i=p({kind:`junction`,id:`junction-${e.ix}-${e.iz}-${r}`,props:{arms:t.arms.length,roads:[...new Set(t.arms.map(e=>e.section.name).filter(Boolean))].join(` · `)||`unnamed roads`,source:f(`roads`)}});a.as(i).on(null),o.as(i).on(null),Fh(a,t,{...Oh,...n.street}),Fh(o,t,{...Oh,...n.street},!0),T.addRing(t.platform);for(let e of t.corners)for(let t of e.bands)t.kind!==`strip`&&T.addRing([...t.inner,...[...t.outer].reverse()])}),w&&(h.walkways=yh(S,C));let E=[];y.forEach((e,t)=>{if(e.cls>=2&&e.cls<=5)for(let n=0;n+1<e.pts.length;n++)E.push({ax:e.pts[n][0],az:e.pts[n][1],bx:e.pts[n+1][0],bz:e.pts[n+1][1],width:e.width,cls:e.cls,sec:S[t]})});let D=[],O=new Map((e.fronts??[]).map(e=>[e[0],[e[1]/10,e[2]/10,e[3]/1e3,e[4]/1e3]])),k=new Map((e.fronts??[]).filter(e=>e.length>=7).map(e=>[e[0],[e[5]/10,e[6]/10]])),A=new Map((e.lots??[]).map(e=>[e[0],og(e,1).map(([e,t])=>[e*5,t*5])]));A.size&&(h.lots=[...A].map(([e,t])=>({building:e,ring:t}))),e.buildings.forEach((t,r)=>{let c=og(t,3);if(c.length>1&&c[0][0]===c[c.length-1][0]&&c[0][1]===c[c.length-1][1]&&(c=c.slice(0,-1)),c.length<3)return;D.push(c);let u=t[0]/10,_=l(t[2]),v=Math.abs(bm(c)),y=g(r),b=0,x=0;for(let[e,t]of c)b+=e,x+=t;b/=c.length,x/=c.length;let S=[b,x],C=1/0,w=e.anchors?.[r];if(w!==void 0&&c[w])S=c[w];else for(let e of c){let t=m(e[0],e[1]);t<C&&(C=t,S=e)}let T=e.homes?.length?e.homes[r]===1:qm(_,v,u);if(!i(`building`)){T&&h.houses.push({x:b,z:x,id:-1,top:u});return}let j=p({kind:`building`,id:`bld-${e.ix}-${e.iz}-${r}`,props:{kind:_||(T?`house`:`building`),"height (m)":u,"height from":sg[t[1]]??`default`,"footprint (m²)":Math.round(v),...d(0,r),source:f(`buildings`)}});a.as(j).on(S),o.as(j).on(S);let M=e.pads?e.pads[2*r]/100:0;if(e.pads){let t=e.pads[2*r+1]/100,i=n.foundation??`#8d877c`;for(let e=0;e<c.length;e++){let[n,r]=c[e],[s,l]=c[(e+1)%c.length];a.wall(n,r,s,l,t,M,i),o.wall(n,r,s,l,t,M,i)}if(T&&t<M){let e=s(c.flat(),[],2),t=n.pad??`#8f8a80`;for(let n=0;n+2<e.length;n+=3){let r=c[e[n]],i=c[e[n+1]],o=c[e[n+2]],s=(i[1]-r[1])*(o[0]-r[0])-(i[0]-r[0])*(o[1]-r[1])>0;a.tri([r[0],M,r[1]],s?[i[0],M,i[1]]:[o[0],M,o[1]],s?[o[0],M,o[1]]:[i[0],M,i[1]],t)}}a.lift(M),o.lift(M)}let N=()=>{a.lift(0),o.lift(0)},P=e.pads&&e.pads[2*r+1]/100<M?0:void 0;if(!T||(h.houses.push({x:b,z:x,id:j,top:M+u}),!n.houses&&v>400)){Xm(a,c,u,y,n.block,!1,P),Xm(o,c,u,g(r),n.block,!0,P),N();return}let F=Cm(b,x,E),I=O.get(r),L=I?(()=>{let[e,t,n,r]=I,i=(e-b)*-r+(t-x)*n>0?1:-1;return{x:-r*i,z:n*i}})():F&&F.d<70?{x:(F.px-b)/F.d,z:(F.pz-x)/F.d}:void 0;if(n.houses){let t=A.get(r),i={footprint:c.map(([e,t])=>[e-b,t-x]),height:u,front:L?[L.x,L.z]:null,seed:e.seeds?.[r]??Em(e.ix,e.iz,r),props:{kind:_},...t?{lot:t.map(([e,t])=>[e-b,t-x])}:{}};a.lift(0),o.lift(0),n.houses.into(a,i,{at:[b,M,x],id:j,anchor:S,seed:i.seed}),n.houses.into(o,i,{at:[b,M,x],id:j,anchor:S,seed:i.seed,level:1})}else Zm(a,{ring:c,height:u,front:L},n.house,y,!1,P),Zm(o,{ring:c,height:u,front:L},n.house,g(r),!0,P);N();let R=O.get(r);if(R){let[e,t,i,o]=R,s=-o,l=i,u=(b-e)*s+(x-t)*l>0?1:-1,d=s*u,f=l*u,p=1/0,m=-1/0;for(let[n,r]of c){let a=(n-e)*i+(r-t)*o;p=Math.min(p,a),m=Math.max(m,a)}p-=1.5,m+=1.5;let h=k.get(r);if(h){if(h[1]-h[0]<1)return;p=h[0],m=h[1]}let g=(n,r)=>[e+i*n+d*r,t+o*n+f*r],_=n.lot,v=y.chance(_.fence.chance)?y.weighted(_.fence.types):`none`,S=y.chance(.5),C=_.fence.driveway,w=1/0,T=-1/0;for(let[n,r]of c){let a=(n-e)*i+(r-t)*o;w=Math.min(w,a),T=Math.max(T,a)}let E=Math.abs(m-T-(w-p))>1?m-T>w-p:S,[D,O]=E?[p,m-C]:[p+C,m];if(O>D&&th(a,g(D,.15),g(O,.15),v,_.fence),y.chance(_.bins.chance)){let e=g((E?m-C/2:p+C/2)+(E?-1:1)*(C/2+1.2),2.2);nh(a,e[0],e[1],{x:i,z:o},_.bins)}_.driveway&&_.driveway.chance>0&&y.chance(_.driveway.chance)&&eh(a,{px:e,pz:t,tx:i,tz:o,nx:d,nz:f},E?m-C/2:p+C/2,c,A.get(r)??null,_.driveway);return}if(!F||!L||F.d>70)return;let z=(b-F.px)*-F.tz+(x-F.pz)*F.tx>0?1:-1,B=-F.tz*z,V=F.tx*z,H=F.seg.sec.profile,U=H.carriageway/2+mm(z>0?H.right:H.left);if(F.d<U+2.5)return;let ee=1/0,W=-1/0;for(let[e,t]of c){let n=(e-F.px)*F.tx+(t-F.pz)*F.tz;ee=Math.min(ee,n),W=Math.max(W,n)}ee-=1.5,W+=1.5;let te=(e,t)=>[F.px+F.tx*e+B*t,F.pz+F.tz*e+V*t],ne=n.lot,re=y.chance(ne.fence.chance)?y.weighted(ne.fence.types):`none`,ie=y.chance(.5),ae=ne.fence.driveway,[oe,se]=ie?[ee,W-ae]:[ee+ae,W];if(se>oe&&th(a,te(oe,U),te(se,U),re,ne.fence),y.chance(ne.bins.chance)){let e=te((ie?W-ae/2:ee+ae/2)+(ie?-1:1)*(ae/2+1.2),U+2.2);nh(a,e[0],e[1],{x:F.tx,z:F.tz},ne.bins)}});for(let e of D)T.addRing(e);let j=e.trees,M=[];for(let t=0;i(`tree`)&&t+3<j.length;t+=4){let n=e.baked?[j[t]/10,j[t+1]/10]:T.place(j[t]/10,j[t+1]/10,{r:.3,nudge:2});if(!n)continue;let[r,i]=n,a=j[t+2]/10,o=j[t+3]/10;M.push({x:r,z:i,height:a,radius:o,seed:e.tseeds?.[t/4]??Em(e.ix,e.iz,t),id:p({kind:`tree`,id:`tree-${e.ix}-${e.iz}-${t/4}`,props:{"height (m)":a,"crown radius (m)":o,...d(1,t/4),source:f(`trees`)}})})}let N=e.furniture??[];if(N.length){h.furniture=[];for(let e=0;e+4<N.length;e+=5){let t=d(2,e/5).placement;h.furniture.push({kind:l(N[e]),x:N[e+1]/10,z:N[e+2]/10,id:l(N[e+3]),seed:N[e+4],...t?{patched:t}:{}})}}e.omitted?.length&&(h.omitted=e.omitted.map(e=>({what:[`building`,`plot`,`tree`,`furniture`][e[0]]??`building`,x:e[1]/10,z:e[2]/10,check:l(e[3]),id:l(e[4])})));let P=Xh(M,n.trees),F=P.map(e=>({shape:e.tree.levels,data:e.data,radius:e.tree.crownRadius*.8}));return{chunk:{mesh:a.build(),far:o.build(),instances:F,info:e=>c[e]??null},extra:h,infos:c,trees:P.map(e=>({key:e.shape,data:e.data}))}}const lg=[`building`,`road`,`tree`,`park`,`water`,`waterway`];function ug(e={}){let t={...ag,...e},n=e=>{let t=111320*Math.cos(e.origin[1]*Math.PI/180);return{ix:e.ix,iz:e.iz,size:e.size,origin:e.origin,at:e.at,lonLat:(n,r)=>[e.origin[0]+(e.at[0]+n)/t,e.origin[1]-(e.at[1]+r)/110574],groundAt:(t,n)=>{let r=Math.max(0,Math.min(e.n-1.001,t/e.step)),i=Math.max(0,Math.min(e.n-1.001,n/e.step)),a=Math.floor(r),o=Math.floor(i),s=r-a,c=i-o,l=(t,n)=>e.heights[n*e.n+t];return(l(a,o)*(1-s)+l(a+1,o)*s)*(1-c)+(l(a,o+1)*(1-s)+l(a+1,o+1)*s)*c}}},r=(e,r,i,a)=>{let o=new Set(i);return cg(e,n(r),t,a,e=>o.has(e))};return{build(e,t,n,i){let a=r(e,t,n,i);return{mesh:/* @__PURE__ */ new Float32Array,far:a.chunk.far,trees:a.trees,infos:a.infos,extra:a.extra}},whole(e,t,n,i){return r(e,t,n,i).chunk.mesh}}}var dg=class extends cm{spans=new at([],{id:`id`});wires;dirtySpans=!1;constructor(e){let t={...ag,...e.look},n=null,r=(r,i)=>{sh(r);let a=n?.area?.sources??{},o=n?.showKind??(()=>!0);if(e.worker){let n=e.worker,s=pg(i),c=lg.filter(e=>o(e)),l=Object.fromEntries(Object.entries(a).map(([e,t])=>[e,{title:t.title,licence:t.licence}]));return n.call(`build`,r,s,c,l).then(e=>({chunk:{mesh:()=>n.call(`whole`,r,s,c,l),far:e.far,instances:e.trees.map(e=>{let n=Yh(t.trees).get(e.key);return{shape:n?.levels??[],data:e.data,radius:(n?.crownRadius??4)*.8}}),info:t=>e.infos[t]??null},extra:e.extra}))}let s=cg(r,i,t,a,o);return{...s,chunk:{...s.chunk,mesh:()=>cg(r,i,t,a,o).chunk.mesh}}};super({lod:[130,300],...e,id:e.id??`streets`,build:r,onTile:(t,r,i)=>{n?.tileChanged(),e.onTile?.(t,r,i)}}),n=this,this.wires=vh({id:`${this.id}-wires`,data:this.spans,color:t.poles.wire,when:e.when??{level:{from:`street`}}})}tileChanged(){this.dirtySpans=!0}showKind=()=>!0;setView(e){return this.showKind=e,this.rebuild()}blockerAt(e,t){let n=this.area;if(!n)return 0;let r=111320*Math.cos(n.origin[1]*Math.PI/180),i=(e-n.origin[0])*r,a=-(t-n.origin[1])*110574,o=Math.floor(i/n.size),s=Math.floor(a/n.size),c=`${o}_${s}`,l=this.blockGrids.get(c);if(!l){let e=this.tiles().find(e=>e.key===c);if(!e)return 0;let t=Math.ceil(n.size/2),r=new Float32Array(t*t);sh(e.tile);for(let n of e.tile.buildings){let e=og(n,3),i=n[0]/10;if(e.length<3)continue;let a=1/0,o=-1/0,s=1/0,c=-1/0;for(let[t,n]of e)a=Math.min(a,t),o=Math.max(o,t),s=Math.min(s,n),c=Math.max(c,n);for(let n=Math.max(0,Math.floor(s/2));n<=Math.min(t-1,Math.floor(c/2));n++)for(let s=Math.max(0,Math.floor(a/2));s<=Math.min(t-1,Math.floor(o/2));s++)wm(s*2+1,n*2+1,e)&&(r[n*t+s]=Math.max(r[n*t+s],i))}l={n:t,h:r},this.blockGrids.set(c,l),this.blockGrids.size>40&&this.blockGrids.delete(this.blockGrids.keys().next().value)}let u=Math.floor((i-o*n.size)/2),d=Math.floor((a-s*n.size)/2);return u<0||d<0||u>=l.n||d>=l.n?0:l.h[d*l.n+u]}blockGrids=/* @__PURE__ */ new Map;frame(e){let t=super.frame(e);if(this.dirtySpans){this.dirtySpans=!1;let e=[];for(let t of this.built())t.extra?.spans.forEach((n,r)=>e.push({id:`${t.key}:${r}`,a:t.ctx.lonLat(n.ax,n.az),b:t.ctx.lonLat(n.bx,n.bz),top:n.top}));this.spans.set(e)}return t}};const fg=e=>new dg(e);function pg(e){let t=e.size/20,n=/* @__PURE__ */ new Float32Array(441);for(let r=0;r<21;r++)for(let i=0;i<21;i++)n[r*21+i]=e.groundAt(i*t,r*t);return{ix:e.ix,iz:e.iz,size:e.size,origin:e.origin,at:e.at,step:t,n:21,heights:n}}var mg=class{points;piece;ux;uz;vx;vz;constructor(e,t,n){this.piece=e,this.points=t;let[r,i,a]=t;this.ux=(i[1]-a[1])/n,this.uz=(a[0]-i[0])/n,this.vx=(a[1]-r[1])/n,this.vz=(r[0]-a[0])/n}height(e,t){let[n,r,i]=this.points,a=e-i[0],o=t-i[1],s=this.ux*a+this.uz*o,c=this.vx*a+this.vz*o;return i[2]+s*(n[2]-i[2])+c*(r[2]-i[2])}contains(e,t){let n=this.points[2],r=e-n[0],i=t-n[1],a=this.ux*r+this.uz*i,o=this.vx*r+this.vz*i;return a>=-1e-7&&o>=-1e-7&&a+o<=1+1e-7}};function hg(e){let t=[],n=/* @__PURE__ */ new WeakMap,r=e=>{let t=n.get(e);return t||(t=e.map(e=>Math.round(e*100)/100),n.set(e,t)),t},i=(e,n)=>{let i=n.map(r),[a,o,s]=i,c=(o[1]-s[1])*(a[0]-s[0])+(s[0]-o[0])*(a[1]-s[1]);Math.abs(c)>1e-8&&t.push(new mg(e,i,c))};for(let t of e)if(t.t===`poly`&&t.fan)for(let e=1;e<t.ring.length;e++)i(t,[t.ring[0],t.ring[e],t.ring[e+1<t.ring.length?e+1:1]]);else if(t.t===`area`||t.t===`poly`){let e=t.t===`area`?t.rings:[t.ring],n=e.flat(),r=[],a=e[0].length;for(let t of e.slice(1))r.push(a),a+=t.length;let o=s(n.flatMap(e=>[Math.round(e[0]*100)/100,Math.round(e[1]*100)/100]),r,2);for(let e=0;e<o.length;e+=3)i(t,o.slice(e,e+3).map(e=>n[e]))}else if(t.t===`strip`)for(let e=1;e<t.edges.length;e++)for(let n=1;n<t.edges[e].length;n++){let r=t.edges[e-1][n-1],a=t.edges[e][n-1],o=t.edges[e][n],s=t.edges[e-1][n];i(t,[r,a,o]),i(t,[r,o,s])}else if(t.t===`box`){let e=Math.cos(t.turn),n=Math.sin(t.turn),r=(r,i)=>[t.at[0]+r*e-i*n,t.at[1]+r*n+i*e,t.top],a=r(-t.d/2,-t.w/2),o=r(t.d/2,-t.w/2),s=r(t.d/2,t.w/2),c=r(-t.d/2,t.w/2);i(t,[a,o,s]),i(t,[a,s,c])}return t}const gg=(e,t,n)=>{let r=e.length,i=e[0],a=e[r-1];if(r<2||a<=i)return t<i||!n&&t===i?0:r;let o=Math.max(0,Math.min(r,Math.floor((t-i)/(a-i)*(r-1)))),s=r=>n?e[r]<=t:e[r]<t;for(;o>0&&!s(o-1);)o--;for(;o<r&&s(o);)o++;return o},_g=(e,t)=>gg(e,t,!1),vg=(e,t)=>gg(e,t,!0);function yg(e,t,n,r,i){for(let a of e)if(a[0]>=t&&a[0]<=r&&a[1]>=n&&a[1]<=i)return!0;for(let a=0;a<e.length;a++){let o=e[a],s=e[(a+1)%e.length],c=s[0]-o[0],l=s[1]-o[1],u=0,d=1,f=!0;for(let[e,a]of[[-c,o[0]-t],[c,r-o[0]],[-l,o[1]-n],[l,i-o[1]]]){if(e===0){if(a<0){f=!1;break}continue}let t=a/e;if(e<0){if(t>d){f=!1;break}t>u&&(u=t)}else{if(t<u){f=!1;break}t<d&&(d=t)}}if(f&&u<=d)return!0}return!1}function bg(e,t={}){e.numericCells&&(e.cells=rh(e.numericCells,e.numericScale??1e3)),e.numericEdges&&(e.edges=rh(e.numericEdges,e.numericScale??1e3));let n=111320*Math.cos(e.origin[1]*Math.PI/180),r=/* @__PURE__ */ new Map,i=(e,t)=>(e+32768)*65536+t+32768,a=(e,t,n,a,o)=>{for(let s=Math.floor(e/50);s<=Math.floor(n/50);s++)for(let e=Math.floor(t/50);e<=Math.floor(a/50);e++){let t=i(s,e),n=r.get(t);n||r.set(t,n={c:[],e:[]}),o(n)}},o=[],s=e=>e.facets??=t.legacy?[]:hg([{t:`area`,kind:`contract`,info:0,rings:e.rings}]);for(let t of e.cells){let e=t[0],n=[],r=1+e;for(let i=0;i<e;i++){let e=[];for(let n=0;n<t[1+i];n++)e.push([t[r],t[r+1],t[r+2]]),r+=3;n.push(e)}let i=1/0,s=1/0,c=-1/0,l=-1/0;for(let e of n[0])i=Math.min(i,e[0]),c=Math.max(c,e[0]),s=Math.min(s,e[1]),l=Math.max(l,e[1]);let u=0;for(let e of n)for(let t=0;t<e.length;t++){let n=e[t],r=e[(t+1)%e.length],i=Math.hypot(r[0]-n[0],r[1]-n[1]);i>.05&&(u=Math.max(u,Math.abs(r[2]-n[2])/i))}let d=o.push({rings:n,box:[i,s,c,l],grade:Math.min(.4,u),facets:null})-1;a(i,s,c,l,e=>e.c.push(d))}let c=[];for(let t of e.edges)for(let e=0;e+5<t.length;e+=3){let n=c.push({ax:t[e],az:t[e+1],ah:t[e+2],bx:t[e+3],bz:t[e+4],bh:t[e+5]})-1,r=c[n];a(Math.min(r.ax,r.bx)-20,Math.min(r.az,r.bz)-20,Math.max(r.ax,r.bx)+20,Math.max(r.az,r.bz)+20,e=>e.e.push(n))}let l=(e,t,n,r=null)=>{let i=!1;for(let a=0,o=n.length-1;a<n.length;o=a++){let s=n[a],c=n[o];if(r!==null){let n=c[0]-s[0],i=c[1]-s[1],a=Math.max(0,Math.min(1,((e-s[0])*n+(t-s[1])*i)/(n*n+i*i||1)));if(Math.hypot(e-s[0]-n*a,t-s[1]-i*a)<=.02)return r}s[1]>t!=c[1]>t&&e<(c[0]-s[0])*(t-s[1])/(c[1]-s[1])+s[0]&&(i=!i)}return i},u=null,d=null,f=()=>{let e=o.map(e=>e.facets??hg([{t:`area`,kind:`contract`,info:0,rings:e.rings}]));d=new Int32Array(o.length+1);let t=0;e.forEach((e,n)=>{d[n]=t,t+=e.length}),d[o.length]=t,u=new Float32Array(t*9);let n=0;for(let t of e)for(let e of t)for(let t of e.points)u[n++]=t[0],u[n++]=t[1],u[n++]=t[2]},p=/* @__PURE__ */ new Float64Array(9),m=/* @__PURE__ */ new Float64Array(9),h=/* @__PURE__ */ new Float64Array(64),g=/* @__PURE__ */ new Float64Array(64),_=/* @__PURE__ */ new Float64Array(64);return{origin:e.origin,kx:n,under(e,n,r={}){if(t.legacy)return;let i=e.xs,a=e.zs,s=i.length,c=a.length;if(s<2||c<2)return;let l=r.margin??.35,v=r.steep??.25,y=r.ground,b=r.coarse??2,x=i[0],S=i[s-1],C=a[0],w=a[c-1],T=(S-x)/(s-1),E=(w-C)/(c-1);u||f(),h.length<2*(s+1)&&(h=new Float64Array(2*(s+1)),g=new Float64Array(2*(s+1)),_=new Float64Array(2*(s+1)));let D=u,O=d;for(let e=0;e<o.length;e++){let t=o[e],u=t.box;if(u[2]<x||u[0]>S||u[3]<C||u[1]>w)continue;let d=t.rings[0][0];if(!r.owns||r.owns(d[0],d[1])){if((u[2]-u[0])*b<=T&&(u[3]-u[1])*b<=E){if(t.low===void 0){let e=1/0;for(let n of t.rings)for(let t of n)t[2]<e&&(e=t[2]);t.low=e}let e=Math.max(0,_g(i,u[0])-1),r=Math.min(s-2,vg(i,u[2])-1),o=Math.max(0,_g(a,u[1])-1),d=Math.min(c-2,vg(a,u[3])-1),f=t.low;for(let c=o;c<=d;c++)for(let u=e;u<=r;u++){if((r>e||d>o)&&!yg(t.rings[0],i[u],a[c],i[u+1],a[c+1]))continue;let p=c*s+u,m=p+1,h=p+s,g=h+1;if(!y){let e=f-l;e<n[p]&&(n[p]=e),e<n[m]&&(n[m]=e),e<n[h]&&(n[h]=e),e<n[g]&&(n[g]=e);continue}let _=i[u],v=i[u+1],b=a[c],x=a[c+1],S=(v-_)/2,C=(x-b)/2,w=_+S,T=b+C,E=y[p],D=y[m],O=y[h],k=y[g],A=(D+k-E-O)/(4*S),j=(O+k-E-D)/(4*C),M=-1/0;for(let e of t.rings)for(let t of e){let e=A*(t[0]-w)+j*(t[1]-T)-t[2];e>M&&(M=e)}let N=-M-f,P=N-A*S-j*C,F=N+A*S-j*C,I=N-A*S+j*C,L=N+A*S+j*C,R=E-f,z=D-f,B=O-f,V=k-f,H=1/0,U=0;for(let e=0;e<=4;e++){let t=e*.25,n=Math.max(R-t*P,z-t*F,B-t*I,V-t*L);n<H-1e-6&&(H=n,U=t)}let ee=f+U*P-l,W=f+U*F-l,te=f+U*I-l,ne=f+U*L-l;ee<n[p]&&(n[p]=ee),W<n[m]&&(n[m]=W),te<n[h]&&(n[h]=te),ne<n[g]&&(n[g]=ne)}}else for(let t=O[e];t<O[e+1];t++){let e=t*9,r=D[e],o=D[e+1],u=D[e+2],d=D[e+3],f=D[e+4],b=D[e+5],T=D[e+6],E=D[e+7],O=D[e+8],k=Math.min(o,f,E),A=Math.max(o,f,E),j=Math.min(r,d,T),M=Math.max(r,d,T);if(A<C||k>w||M<x||j>S)continue;let N=d-r,P=f-o,F=T-r,I=E-o,L=N*I-F*P;if(Math.abs(L)<1e-9)continue;let R=((b-u)*I-(O-u)*P)/L,z=(N*(O-u)-F*(b-u))/L,B=Math.hypot(R,z)<=v,V=Math.max(0,_g(a,k)-1),H=Math.min(c-2,vg(a,A)-1);for(let e=V;e<=H;e++){let t=a[e],c=a[e+1],v=j,C=M,w=Math.min(u,b,O),D=3;if(p[0]=r,m[0]=o,p[1]=d,m[1]=f,p[2]=T,m[2]=E,k<t||A>c){D=0;for(let e=0;e<3;e++){let n=e===0?r:e===1?d:T,i=e===0?o:e===1?f:E,a=e===0?d:e===1?T:r,s=e===0?f:e===1?E:o;i>=t&&i<=c&&(p[D]=n,m[D++]=i),(i-t)*(s-t)<0&&(p[D]=n+(a-n)*(t-i)/(s-i),m[D++]=t),(i-c)*(s-c)<0&&(p[D]=n+(a-n)*(c-i)/(s-i),m[D++]=c)}if(!D)continue;v=1/0,C=-1/0,w=1/0;for(let e=0;e<D;e++){let t=p[e],n=u+R*(t-r)+z*(m[e]-o);t<v&&(v=t),t>C&&(C=t),n<w&&(w=n)}if(C<x||v>S)continue}let N=Math.max(0,_g(i,v)-1),P=Math.min(s-2,vg(i,C)-1)-N+2,F=0;for(let t=0;t<2;t++){let n=a[e+t]-o,c=(e+t)*s+N;for(let e=0;e<P;e++){let a=u+R*(i[N+e]-r)+z*n-w;h[t*P+e]=a,y&&(g[t*P+e]=y[c+e]-w),a>F?F=a:-a>F&&(F=-a)}}let I=+!!B,L=!1;if(y&&F>.01){let e=1/0;for(let t=0;t<=4;t++){let n=1-t*.25,r=-1/0;for(let e=0;e<2*P;e++){let t=g[e]-n*h[e];t>r&&(r=t)}r<e-1e-6&&(e=r,I=n)}let n=(i[N]+i[N+P-1])/2,a=(t+c)/2,s=0,l=0,d=0,f=0;for(let e=0;e<P;e++){let t=i[N+e]-n,r=g[e],a=g[P+e];s+=r,l+=a,d+=2*t*t,f+=t*(r+a)}let v=(s+l)/(2*P),y=d>0?f/d:0,b=(l-s)/P/(c-t),x=-1/0;for(let e=0;e<D;e++){let t=v+y*(p[e]-n)+b*(m[e]-a)-(u+R*(p[e]-r)+z*(m[e]-o)-w);t>x&&(x=t)}let S=-1/0;for(let e=0;e<2;e++){let r=(e?c:t)-a;for(let t=0;t<P;t++){let a=v+y*(i[N+t]-n)+b*r-x;_[e*P+t]=a;let o=g[e*P+t]-a;o>S&&(S=o)}}S<e-1e-6&&(L=!0)}for(let t=0;t<2;t++){let r=(e+t)*s+N;for(let e=0;e<P;e++){let i=w+(L?_[t*P+e]:I*h[t*P+e])-l;i<n[r+e]&&(n[r+e]=i)}}}}}}},limit(e,n,a,u,d=0,f=!0){let p=r.get(i(Math.floor(e/50),Math.floor(n/50)));if(!p)return null;let m=1/0;if(f)for(let r of p.c){let i=o[r];if(e<i.box[0]||e>i.box[2]||n<i.box[1]||n>i.box[3]||!l(e,n,i.rings[0],!t.legacy||null)||i.rings.slice(1).some(r=>l(e,n,r,t.legacy?null:!1)))continue;let a=0,c=0,u=1/0;for(let t of i.rings)for(let r of t){let t=1/(Math.hypot(r[0]-e,r[1]-n)**2+.01);a+=t,c+=t*r[2],u=Math.min(u,r[2])}let f=s(i).find(t=>t.contains(e,n)),p=(f?f.height(e,n):Math.min(c/a,u+.3))-((t.legacy?.3:.35)+i.grade*d*1.6);p<m&&(m=p)}for(let t of p.e){let r=c[t];if(e<(r.ax<r.bx?r.ax:r.bx)-u||e>(r.ax>r.bx?r.ax:r.bx)+u||n<(r.az<r.bz?r.az:r.bz)-u||n>(r.az>r.bz?r.az:r.bz)+u)continue;let i=r.bx-r.ax,o=r.bz-r.az,s=i*i+o*o||1,l=Math.max(0,Math.min(1,((e-r.ax)*i+(n-r.az)*o)/s)),f=Math.hypot(e-r.ax-i*l,n-r.az-o*l);if(f>u)continue;let p=r.ah+(r.bh-r.ah)*l,h=Math.min(.4,Math.abs(r.bh-r.ah)/Math.max(.1,Math.hypot(i,o)))*d*1.6,g=f<=4?p+.06-h:p+.06-h+(f-4)*a;g<m&&(m=g)}return Number.isFinite(m)?m:null}}}const xg={water:`#477985`,viaduct:`#b5b1a7`,carriageway:`#3b3e42`,platform:`#3b3e42`,edge:`#9a978d`,deck:`#b5b1a7`,cut:`#a8a49a`,footpath:`#c3beb2`,verge:`#7f9f58`,ramp:`#b7b4ab`,kerb:`#b7b4ab`,"kerb-face":`#a9a69c`,skirt:`#9a978d`,median:`#6e9b46`,island:`#6e9b46`,marking:`#f2f1ea`,parapet:`#c9c6bd`,"parapet-top":`#d4d1c8`,"deck-side":`#b5b1a7`,"deck-under":`#8f8c84`,"pier-cap":`#a8a49a`,"pier-cap-under":`#96938b`,girder:`#aaa79e`,"girder-under":`#96938b`,pier:`#a8a49a`,abutment:`#a8a49a`,retaining:`#a8a49a`,"ramp-wall":`#b0ada4`,portal:`#9d9a91`};function Sg(){let e=/* @__PURE__ */ new Map,t=(t,n,r,i)=>{let a=`${+!!i}|${r.join(`,`)}|${t}`,o=e.get(a);return o?(e.delete(a),e.set(a,o),o):(o=bg({...n,origin:r},{legacy:i}),e.set(a,o),e.size>96&&e.delete(e.keys().next().value),o)};return{roads:(e,t,n,r)=>wg({...e,infos:[],pieces:e.pieces??[]},t,n,new Set(r)).mesh,clamp:e=>Ag(e.out,e.t,e.n,e.at,e.keys,e.tiles.map(n=>n?t(n.url,n.j,e.at.origin,e.o.legacy):null),e.o)}}const Cg=e=>({v:e.v,ix:e.ix,iz:e.iz,size:e.size,...e.numeric?{numeric:e.numeric,numericScale:e.numericScale,turnUnits:e.turnUnits}:{pieces:e.pieces}});function wg(e,t,n,r=/* @__PURE__ */ new Set){ih(e);let i={};for(let e in n)i[e]=W(n[e]);let a=new ce,o=t.indexOf(`kerb`),c=/* @__PURE__ */ new Map,l=(e,t)=>{let n=4+t*6,r=e=>Math.round(e*100);return`${r(e[n])},${r(e[n+1])},${r(e[n]+e[n+4])},${r(e[n+1]+e[n+5])},${r(e[n+2])}`};if(o>=0&&!r.has(`kerb`)){for(let t of e.pieces)if(t[0]===5&&t[1]===o&&t[3]>1)for(let e of[0,t[3]-1]){let n=l(t,e);c.set(n,(c.get(n)??0)+1)}}for(let n of e.pieces){let o=n[0],u=t[n[1]]??``,d=n[2];if(r.has(u))continue;let f=i[u]??8947848;if(a.as(d).material([`viaduct`,`deck`,`deck-under`,`deck-side`,`girder`,`girder-under`,`parapet`,`pier`,`pier-cap`,`pier-cap-under`,`abutment`,`retaining`,`ramp-wall`,`portal`,`cut`].includes(u)?7:0),o===0){let e=n[3],t=n[4],r=(e,r)=>{let i=5+(e*t+r)*3;return[n[i],n[i+2],n[i+1]]};for(let n=0;n+1<e;n++)for(let e=0;e+1<t;e++){let t=r(n,e),i=r(n+1,e),o=r(n+1,e+1),s=r(n,e+1);u===`deck-under`||u===`girder-under`?(i[2]-t[2])*(o[0]-t[0])-(i[0]-t[0])*(o[2]-t[2])>=0?a.quad(s,o,i,t,f):a.quad(t,i,o,s,f):a.quadUp(t,i,o,s,f)}}else if(o===1){let e=n[3]===1,t=n[4],r=e=>[n[5+e*3],n[7+e*3],n[6+e*3]];if(e)for(let e=1;e<t;e++)a.triUp(r(0),r(e),r(e+1<t?e+1:1),f);else{let e=[];for(let r=0;r<t;r++)e.push(n[5+r*3],n[6+r*3]);let i=s(e,[],2);for(let e=0;e+2<i.length;e+=3)a.triUp(r(i[e]),r(i[e+1]),r(i[e+2]),f)}}else if(o===2){let e=n[3],t=e=>4+e*4;for(let r=0;r+1<e;r++){let e=t(r),i=t(r+1);a.quad([n[e],n[e+3],n[e+1]],[n[i],n[i+3],n[i+1]],[n[i],n[i+2],n[i+1]],[n[e],n[e+2],n[e+1]],f)}}else if(o===4){let e=n[3]===1,t=n[4],r=[],i=[],o=[],c=5+t;for(let e=0;e<t;e++){let t=n[5+e];e&&o.push(r.length/2);for(let e=0;e<t;e++)r.push(n[c],n[c+1]),i.push(n[c+2]),c+=3}let l=e=>[r[e*2],i[e],r[e*2+1]],u=s(r,o,2);for(let t=0;t+2<u.length;t+=3){let n=l(u[t]),r=l(u[t+1]),i=l(u[t+2]);e?(r[2]-n[2])*(i[0]-n[0])-(r[0]-n[0])*(i[2]-n[2])>=0?a.tri(n,i,r,f):a.tri(n,r,i,f):a.triUp(n,r,i,f)}}else if(o===5){let t=n[3],r=e=>{let t=4+e*6;return{x:n[t],z:n[t+1],h:n[t+2],g:n[t+3],ix:n[t]+n[t+4],iz:n[t+1]+n[t+5]}},o=(e,t,n,r,i,o,s,c,l)=>a.quad([e,r,t],[i,c,o],[i,s,o],[e,n,t],l);for(let n=0;n+1<t;n++){let t=r(n),s=r(n+1);if(u===`ramp`)a.quadUp([t.x,t.h+.02,t.z],[s.x,s.h+.02,s.z],[s.ix,s.h+.02,s.iz],[t.ix,t.h+.02,t.iz],i.ramp??f),o(t.x,t.z,t.h+.02,Math.min(t.g,t.h)-.6,s.x,s.z,s.h+.02,Math.min(s.g,s.h)-.6,i.skirt??f);else if(u===`kerb`){let e=i.kerb??f,n=i[`kerb-face`]??f,r=i.skirt??f;a.quadUp([t.x,t.h+.15,t.z],[s.x,s.h+.15,s.z],[s.ix,s.h+.15,s.iz],[t.ix,t.h+.15,t.iz],e),o(t.ix,t.iz,t.h+.15,t.h-.05,s.ix,s.iz,s.h+.15,s.h-.05,n),o(t.x,t.z,t.h+.15,Math.min(t.g,t.h)-.6,s.x,s.z,s.h+.15,Math.min(s.g,s.h)-.6,r)}else if(u===`viaduct`||u===`deck`){let n=u===`viaduct`?.9:1;a.quadUp([t.x,t.h+n,t.z],[s.x,s.h+n,s.z],[s.ix,s.h+n,s.iz],[t.ix,t.h+n,t.iz],i[`parapet-top`]??f),o(t.ix,t.iz,t.h+n,t.h,s.ix,s.iz,s.h+n,s.h,i.parapet??f),o(t.x,t.z,t.h+n,t.h,s.x,s.z,s.h+n,s.h,i.parapet??f),o(t.x,t.z,t.h,t.h-(e.v<=2?1.2:.35),s.x,s.z,s.h,s.h-(e.v<=2?1.2:.35),i[`deck-side`]??f)}else u===`cut`?o(t.x,t.z,Math.max(t.h+.6,t.g+.3),t.h-.05,s.x,s.z,Math.max(s.h+.6,s.g+.3),s.h-.05,i.retaining??f):o(t.x,t.z,t.h,Math.min(t.g,t.h)-.6,s.x,s.z,s.h,Math.min(s.g,s.h)-.6,i.skirt??f)}if(u===`kerb`&&t>1)for(let[e,o]of[[0,1],[t-1,t-2]]){if((c.get(l(n,e))??0)>1)continue;let t=r(e),s=r(o),u=[[t.x,t.h+.15,t.z],[t.ix,t.h+.15,t.iz],[t.ix,t.h-.05,t.iz],[t.x,Math.min(t.g,t.h)-.6,t.z]],d=u[1][0]-u[0][0],p=u[1][2]-u[0][2],m=u[2][1]-u[0][1];d*m*(t.z-s.z)-p*m*(t.x-s.x)>=0?a.quad(u[0],u[1],u[2],u[3],i[`kerb-face`]??f):a.quad(u[3],u[2],u[1],u[0],i[`kerb-face`]??f)}}else if(o===3){let[e,t,r,i,o,s,c]=n.slice(3),l=Math.cos(c),u=Math.sin(c),d=(n,r,i)=>[e+n*l-r*u,i,t+n*u+r*l],p=[d(-s/2,-o/2,0),d(s/2,-o/2,0),d(s/2,o/2,0),d(-s/2,o/2,0)];for(let e=0;e<4;e++){let t=p[e],n=p[(e+1)%4];a.quad([n[0],r,n[2]],[t[0],r,t[2]],[t[0],i,t[2]],[n[0],i,n[2]],f)}a.quadUp([p[0][0],i,p[0][2]],[p[1][0],i,p[1][2]],[p[2][0],i,p[2][2]],[p[3][0],i,p[3][2]],f)}}return{mesh:a.build(),ids:e.infos.length}}const Tg=/* @__PURE__ */ new Set([`water`,`deck-under`,`girder-under`,`pier-cap`,`pier-cap-under`]);function Eg(e,t,n=/* @__PURE__ */ new Set){ih(e);let r=[],i=(e,t,n,i,a,o,s,c,l)=>{r.push(e,t,n,i,a,o,s,c,l)};for(let r of e.pieces){let e=r[0],a=t[r[1]]??``;if(!(n.has(a)||Tg.has(a))){if(e===0){let e=r[3],t=r[4],n=(e,n)=>5+(e*t+n)*3;for(let a=0;a+1<e;a++)for(let e=0;e+1<t;e++){let t=n(a,e),o=n(a+1,e),s=n(a+1,e+1),c=n(a,e+1);i(r[t],r[t+1],r[t+2],r[o],r[o+1],r[o+2],r[s],r[s+1],r[s+2]),i(r[t],r[t+1],r[t+2],r[s],r[s+1],r[s+2],r[c],r[c+1],r[c+2])}}else if(e===1||e===4&&r[3]!==1){let t=e===1?1:r[4],n=[],a=[],o=[],c=e===1?5:5+t;for(let i=0;i<t;i++){let t=e===1?r[4]:r[5+i];i&&o.push(n.length/2);for(let e=0;e<t;e++)n.push(r[c],r[c+1]),a.push(r[c+2]),c+=3}let l=e=>[n[e*2],n[e*2+1],a[e]],u=a.length,d=[];if(e===1&&r[3]===1)for(let e=1;e<u;e++)d.push(0,e,e+1<u?e+1:1);else d.push(...s(n,o,2));for(let e=0;e+2<d.length;e+=3)i(...l(d[e]),...l(d[e+1]),...l(d[e+2]))}else if(e===5&&(a===`kerb`||a===`ramp`)){let e=r[3],t=a===`kerb`?.15:.02;for(let n=0;n+1<e;n++){let e=4+n*6,a=e+6,o=r[e+2]+t,s=r[a+2]+t;i(r[e],r[e+1],o,r[a],r[a+1],s,r[a]+r[a+4],r[a+1]+r[a+5],s),i(r[e],r[e+1],o,r[a]+r[a+4],r[a+1]+r[a+5],s,r[e]+r[e+4],r[e+1]+r[e+5],o)}}else if(e===3&&a===`ramp`){let[e,t,,n,a,o,s]=r.slice(3),c=Math.cos(s),l=Math.sin(s),u=(r,i)=>[e+r*c-i*l,t+r*l+i*c,n];i(...u(-o/2,-a/2),...u(o/2,-a/2),...u(o/2,a/2)),i(...u(-o/2,-a/2),...u(o/2,a/2),...u(-o/2,a/2))}}}return new Float32Array(r)}var Dg=class extends cm{constructor(e){let t={...xg,...e.look},n=new Set(e.hide??[]),r=null,i=(i,a)=>{let o=r?.area?.kinds??[],s=((r?.area)?.report?.stages??[]).flatMap(e=>e.patches??[]);for(let e of i.infos)e.patchRefs&&(e.props.patches=e.patchRefs.map(e=>s[e]),delete e.patchRefs);let c=e=>{let t=i.infos[e];return t?{kind:t.kind,id:`${i.ix}_${i.iz}:${e}`,props:t.props}:null};if(e.worker)return e.worker.call(`roads`,Cg(i),o,t,[...n]).then(e=>({chunk:{mesh:e,free:!0,info:c}}));ih(i);let{mesh:l}=wg(i,o,t,n);return{chunk:{mesh:l,free:!0,info:c}}};super({lod:[400,1200],hover:!1,...e,id:e.id??`roads`,build:i}),r=this,this.hidden=n}hidden;grids=/* @__PURE__ */ new WeakMap;standVersion=0;put(e,t){return this.standVersion++,super.put(e,t),this.reach(e),this}drop(e){return this.standVersion++,this.unreach(e),super.drop(e)}reaching=/* @__PURE__ */ new Map;reached=/* @__PURE__ */ new Map;reach(e){this.unreach(e);let t=this.area,[n,r]=e.split(`_`).map(Number);if(!t)return;let i=this.reachOf(e),a=t.size,o=[],[s,c,l,u]=i?[n+Math.floor(i[0]/a),n+Math.floor(i[3]/a),r+Math.floor(i[2]/a),r+Math.floor(i[5]/a)]:[n,n,r,r];for(let t=Math.min(s,n);t<=Math.max(c,n);t++)for(let i=Math.min(l,r);i<=Math.max(u,r);i++){let a=`${t}_${i}`,s=this.reaching.get(a);s||this.reaching.set(a,s=/* @__PURE__ */ new Map),s.set(e,[n,r]),o.push(a)}this.reached.set(e,o)}unreach(e){for(let t of this.reached.get(e)??[]){let n=this.reaching.get(t);n?.delete(e),n&&!n.size&&this.reaching.delete(t)}this.reached.delete(e)}levelsAt(e,t,n){let r=this.area;if(!r)return;let i=111320*Math.cos(r.origin[1]*Math.PI/180),a=(e-r.origin[0])*i,o=-(t-r.origin[1])*110574,s=r.size,c=this.reaching.get(`${Math.floor(a/s)}_${Math.floor(o/s)}`);if(c)for(let[e,[t,i]]of c){let c=this.builtTile(t,i);if(!c)continue;let l=a-t*s,u=o-i*s,d=this.reachOf(e);if(d&&(l<d[0]-.1||l>d[3]+.1||u<d[2]-.1||u>d[5]+.1))continue;let f=this.grids.get(c);if(!f){let e=r.kinds??[];this.grids.set(c,f=ro(Eg({...c},e,this.hidden)))}f.levelsAt(l,u,n)}}};const Og=e=>new Dg(e),kg=e=>{let[,t=``,n=e]=/^([a-z]+:\/\/[^/]*)?(.*)$/i.exec(e)??[],r=[];for(let e of n.split(`/`))e===`..`?r.pop():e!==`.`&&r.push(e);return t+r.join(`/`)};function Ag(e,t,n,r,i,a,o){let{batter:s,reach:c,under:l}=o,[u,d]=I(t.z,t.x,t.y,0,0),[f,p]=I(t.z,t.x,t.y,1,1),m=e=>(e-r.origin[0])*r.kx,h=e=>-(e-r.origin[1])*110574,g=/* @__PURE__ */ new Map;i.forEach((e,t)=>{let n=a[t];if(n){let[t,r]=e.split(`_`).map(Number);g.set(t*65536+r,n)}});let _=o.legacy||l?0:Math.max(Math.abs(m(f)-m(u)),Math.abs(h(p)-h(d)))/(n-1);for(let i=0;i<n;i++)for(let a=0;a<n;a++){let[o,u]=I(t.z,t.x,t.y,a/(n-1),i/(n-1)),d=m(o),f=h(u),p=g.get(Math.floor(d/r.size)*65536+Math.floor(f/r.size))?.limit(d,f,s,c,_,!l)??1/0;if(p<1/0){let t=i*n+a;p<e[t]&&(e[t]=p)}}if(l&&a.some(Boolean)){let o=new Float64Array(n),s=new Float64Array(n),c=new Float32Array(n*n).fill(1/0),u=new Set(i.filter((e,t)=>a[t]).map(e=>{let[t,n]=e.split(`_`).map(Number);return t*65536+n}));for(let e=0;e<n;e++)o[e]=m(I(t.z,t.x,t.y,e/(n-1),0)[0]);for(let e=0;e<n;e++)s[e]=h(I(t.z,t.x,t.y,0,e/(n-1))[1]);i.forEach((t,n)=>{let i=a[n];if(!i)return;let[d,f]=t.split(`_`).map(Number);i.under({xs:o,zs:s},c,{...l,ground:e,owns:(e,t)=>{let n=Math.floor(e/r.size),i=Math.floor(t/r.size);return n===d&&i===f||!u.has(n*65536+i)}})});for(let t=0;t<n*n;t++)c[t]<e[t]&&(e[t]=c[t])}return e}function jg(e,t,n={}){let r=n.batter??.5,i=n.reach??14,a=n.under===!1||n.legacy?null:{...n.under},o={batter:r,reach:i,legacy:!!n.legacy,under:a?{margin:a.margin,steep:a.steep,coarse:a.coarse}:null},s=(e,t)=>e.loader.get(t,{kind:`json`}),c=null,l=/* @__PURE__ */ new Map,u=t.replace(/[^/]*$/,``),d=e=>c??=s(e,t).then(async t=>t?{origin:t.origin,size:t.size,tiles:new Set(t.tiles.map(e=>`${e[0]}_${e[1]}`)),kx:111320*Math.cos(t.origin[1]*Math.PI/180),from:t.tilesFrom?kg(`${u}${t.tilesFrom}`):u,...t.terrain?{terrain:await s(e,`${u}${t.terrain}`)}:{}}:null),f=(e,t,r,i)=>{let a=l.get(t);return a||(a=s(e,`${i}clamp/${t}.json`).then(e=>e?bg({...e,origin:r},{legacy:n.legacy}):null),l.set(t,a)),a};return{kind:`terrain`,id:`${e.id}-clamped`,attribution:e.attribution,terms:e.terms,when:e.when,where:e.where,maxZoom:Math.max(e.maxZoom,20),blend:e.blend,get version(){return e.version},urlFor:e.urlFor?t=>e.urlFor(t):void 0,async heights(t,r,i){let[a,c]=await Promise.all([e.heights(t,r,i),d(i)]);if(c?.terrain?.numeric&&(c.terrain.data=Float32Array.from(rh(c.terrain.numeric,100)[0]),delete c.terrain.numeric),!a||!c)return a;let[l,u]=I(t.z,t.x,t.y,0,0),[p,m]=I(t.z,t.x,t.y,1,1),h=e=>(e-c.origin[0])*c.kx,g=e=>-(e-c.origin[1])*110574,_=[];for(let e=Math.floor(Math.min(h(l),h(p))/c.size);e<=Math.floor(Math.max(h(l),h(p))/c.size);e++)for(let t=Math.floor(Math.min(g(u),g(m))/c.size);t<=Math.floor(Math.max(g(u),g(m))/c.size);t++){let n=`${e}_${t}`;c.tiles.has(n)&&_.push(n)}if(!_.length&&!c.terrain)return a;let v=n.worker,y=v?await Promise.all(_.map(e=>s(i,`${c.from}clamp/${e}.json`))):null,b=y?null:await Promise.all(_.map(e=>f(i,e,c.origin,c.from))),x=(y??b).some(Boolean);if(!x&&!c.terrain)return a;let S=a.slice();if(c.terrain)for(let e=0;e<r;e++)for(let n=0;n<r;n++){let[i,a]=I(t.z,t.x,t.y,n/(r-1),e/(r-1)),o=h(i),s=g(a),l=c.terrain,u=(o-l.x0)/l.step,d=(s-l.z0)/l.step;if(u>=0&&d>=0&&u<l.nx-1&&d<l.nz-1){let t=Math.floor(u),i=Math.floor(d),a=u-t,o=d-i,s=l.data,c=i*l.nx+t;S[e*r+n]=(s[c]*(1-a)+s[c+1]*a)*(1-o)+(s[c+l.nx]*(1-a)+s[c+l.nx+1]*a)*o}}if(!x)return S;let C={origin:c.origin,size:c.size,kx:c.kx};return v&&y?v.callIn(0,`clamp`,{out:S,t:{z:t.z,x:t.x,y:t.y},n:r,at:C,keys:_,tiles:y.map((e,t)=>e?{url:`${c.from}clamp/${_[t]}.json`,j:e}:null),o}):Ag(S,t,r,C,_,b,o)}}}const Mg=Math.PI/180,Ng=110574;function Pg(e,t){let n=111320*Math.cos(t[1]*Mg),r=(e,r)=>[(e-t[0])*n,-(r-t[1])*Ng],i=(e,r)=>[t[0]+e/n,t[1]-r/Ng],a=/* @__PURE__ */ new Map;for(let t of e)t.nodes.forEach((e,n)=>a.set(e,(a.get(e)??0)+(n===0||n===t.nodes.length-1?2:1)));let o=/* @__PURE__ */ new Map,s=/* @__PURE__ */ new Map,c=(e,t)=>{let n=o.get(e);return n||o.set(e,n={id:e,at:t,edges:[]}),n};for(let t of e){let e=t.tags,n=Number.parseInt(e.layer??``,10),i=!!e.bridge&&e.bridge!==`no`,o=!!e.tunnel&&e.tunnel!==`no`&&e.tunnel!==`building_passage`,l=Number.isFinite(n)?n:i?1:o?-1:0,u=e.oneway===`yes`||e.oneway===`1`||e.oneway===`-1`||e.junction===`roundabout`||e.highway===`motorway`||(e.highway?.endsWith(`_link`)??!1)&&e.oneway!==`no`,d=e.oneway===`-1`,f=d?[...t.nodes].reverse():t.nodes,p=d?[...t.pts].reverse():t.pts,m=0,h=0;for(let n=1;n<f.length;n++){if(n<f.length-1&&(a.get(f[n])??0)<2)continue;let d=p.slice(m,n+1).map(([e,t])=>r(e,t)),g=f.slice(m,n+1);if(d.length>=2){let n=`${t.id}:${h++}`,r=g[0],a=g[g.length-1];s.set(n,{id:n,way:t.id,tags:e,a:r,b:a,pts:d,nodes:g,layer:l,bridge:i,tunnel:o,oneway:u}),c(r,d[0]).edges.push(n),c(a,d[d.length-1]).edges.push(n)}m=n}}return{origin:t,nodes:o,edges:s,toLocal:r,toLonLat:i}}const Fg={motorway:0,trunk:1,primary:2,secondary:3,tertiary:4,unclassified:5,residential:5,living_street:5,road:5,service:7},Ig=e=>Fg[(e.highway??``).replace(/_link$/,``)]??6,Lg=e=>String(e.highway??``).replace(/_link$/,``)in Fg&&e.area!==`yes`&&e.service!==`parking_aisle`&&e.service!==`driveway`;var Rg=class extends Zt{stands=!1;rp;src=null;seen=-1;off=null;secs=[];juncs=[];net=null;lines=[];constructor(e){super({hover:{color:`#ffd166`},...e}),this.rp=e}async attach(e){await super.attach(e);let t=this.rp.data;t instanceof Te?(this.src=t,this.off=t.watch(()=>this.build()),t.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.build())):this.build()}detach(){this.off?.(),this.off=null,super.detach()}sections(){return this.secs}junctions(){return this.juncs}network(){return this.net??=td({lines:this.lines,snap:1})}build(){if(this.src&&this.src.version===this.seen)return;this.seen=this.src?.version??0;for(let e of this.keys())this.drop(e);let e=(this.src?this.src.features():this.rp.data).map(e=>e.props?e:{...e,props:e.properties??{}}),t=/* @__PURE__ */ new Map,n=/* @__PURE__ */ new Map,r=[],i=e=>{let t=`${Math.round(e[0]*1e7)},${Math.round(e[1]*1e7)}`,r=n.get(t);return r===void 0&&n.set(t,r=-1-n.size),r};for(let n of e){let e=n.geometry,a=n.props;if(e&&(this.rp.keep?this.rp.keep(n):Lg(a)))for(let o of e.type===`LineString`?[e.coordinates]:e.type===`MultiLineString`?e.coordinates:[]){if(o.length<2)continue;let s=`${n.id??r.length}:${r.length}`,c=Array.isArray(a.nodes)&&a.nodes.length===o.length&&e.type===`LineString`;t.set(s,n),r.push({id:s,nodes:c?a.nodes:o.map(i),pts:o,tags:Object.fromEntries(Object.entries(a).filter(([,e])=>typeof e==`string`||typeof e==`number`).map(([e,t])=>[e,String(t)]))})}}if(this.secs=[],this.juncs=[],this.lines=[],this.net=null,!r.length)return;let a=[r[0].pts[0][0],r[0].pts[0][1]],o=Pg(r,a),s=this.rp.rules??vm,c=/* @__PURE__ */ new Map;for(let e of o.edges.values()){if(e.tunnel)continue;let n=t.get(e.way),r=this.rp.cls?.(n)??Ig(e.tags),i=ym(r,{tags:e.tags},s);this.rp.section&&(i=this.rp.section(n,i));let a={id:e.id,cls:r,name:e.tags.name??``,pts:e.pts,trim:[0,0],ends:[null,null],profile:i};this.secs.push(a),c.set(a,n),this.lines.push({line:e.pts.map(([e,t])=>o.toLonLat(e,t)),props:{cls:r,name:a.name}})}this.juncs=Bh(this.secs,this.rp.junctions??{});let l={...Oh,...this.rp.look},u=this.rp.solve!==!1,d=/* @__PURE__ */ new Map,f=(e,t)=>`${Math.floor(e/500)}_${Math.floor(t/500)}`;for(let e of this.secs){let t=hm(e.pts,gm(e.pts)/2),n=f(t.x,t.z),r=d.get(n);r||d.set(n,r=[]),r.push({kind:`section`,s:e})}for(let e of this.juncs){let t=f(e.at[0],e.at[1]),n=d.get(t);n||d.set(t,n=[]),n.push({kind:`junction`,j:e})}for(let[e,t]of d){let[n,r]=e.split(`_`).map(Number),i=n*500,o=r*500,s=new ce,d=[],f=[];t.forEach((e,t)=>{if(s.as(t),e.kind===`section`){let n=e.s,r=c.get(n),a=n.profile;Nh(s,n,l,!0),d[t]={id:n.id,props:{name:n.name||r.props.highway,highway:r.props.highway,class:n.cls,"carriageway (m)":+a.carriageway.toFixed(1),lanes:a.lanes,profile:a.from}};let u=hm(n.pts,gm(n.pts)/2);f[t]=[u.x-i,u.z-o]}else Fh(s,e.j,l,!0),d[t]={id:`junction ${e.j.key}`,props:{name:`junction`,arms:e.j.arms.length,crossings:e.j.crossings.length}},f[t]=[e.j.at[0]-i,e.j.at[1]-o]});let p=()=>{let e=new ce;return t.forEach((t,n)=>{e.as(n),t.kind===`section`?Nh(e,t.s,l):Fh(e,t.j,l)}),m(e.build(),!0)},m=(e,t)=>{for(let n=0;n+8<=e.length;n+=8){e[n]=e[n]-i,e[n+2]=e[n+2]-o;let r=e[n+7]%65536,a=f[r];!u&&t&&a?(e[n+4]=a[0],e[n+5]=a[1]):(e[n+4]=e[n+4]-i,e[n+5]=e[n+5]-o)}return e};this.put(e,{origin:a,side:500,at:[i,o],mesh:p,far:m(s.build(),!0),info:e=>{let t=d[e];return t?{kind:`road`,id:t.id,props:t.props}:null}})}}};const zg=e=>new Rg(e),Bg=110574,Vg=[`roof:shape`,`roof_shape`,`roofShape`],Hg={flat:`flat`,gabled:`gable`,gable:`gable`,hipped:`hip`,hip:`hip`,"half-hipped":`hip`,pyramidal:`pyramidal`,skillion:`skillion`,lean_to:`skillion`,dome:`dome`,onion:`dome`,round:`dome`,gambrel:`gable`,mansard:`hip`,saltbox:`gable`};function Ug(e){for(let t of Vg){let n=e[t];if(typeof n==`string`&&Hg[n.toLowerCase()])return Hg[n.toLowerCase()]}return null}const Wg=e=>{let t=typeof e==`number`?e:typeof e==`string`?parseFloat(e):NaN;return Number.isFinite(t)?t:null};var Gg=class extends Zt{bp;src=null;seen=-1;off=null;constructor(e){let t=e.windows;super({hover:{color:`#ffd166`},...e,windows:t===!1?void 0:t?{lit:t.lit,interior:t.interior}:void 0}),this.bp=e}async attach(e){await super.attach(e);let t=this.bp.data;t instanceof Te?(this.src=t,this.off=t.watch(()=>this.build()),t.start({loader:e.loader,error:t=>e.world.error(t)}).then(()=>this.build())):this.build()}detach(){this.off?.(),this.off=null,super.detach()}refresh(){return this.seen=-1,this.build(),this}build(){if(this.src&&this.src.version===this.seen)return;this.seen=this.src?.version??0;let e=(this.src?this.src.features():this.bp.data).map(e=>e.props?e:{...e,props:e.properties??{}});for(let e of this.keys())this.drop(e);let t=[];for(let n of e){let e=n.geometry;if(e){if(e.type===`Polygon`)t.push({f:n,rings:e.coordinates});else if(e.type===`MultiPolygon`)for(let r of e.coordinates)t.push({f:n,rings:r})}}if(!t.length)return;let n=t[0].rings[0][0],r=[n[0],n[1]],i=111320*Math.cos(r[1]*G),a=/* @__PURE__ */ new Map;for(let e of t){let t=0,n=0,o=e.rings[0];for(let[e,a]of o)t+=(e-r[0])*i,n+=-(a-r[1])*Bg;let s=`${Math.floor(t/o.length/500)}_${Math.floor(n/o.length/500)}`,c=a.get(s);c||a.set(s,c=[]),c.push(e)}for(let[e,t]of a){let[n,a]=e.split(`_`).map(Number),o=new ce,s=[],c=t.map(e=>{let t=e.rings[0].map(([e,t])=>[(e-r[0])*i-n*500,-(t-r[1])*Bg-a*500]);return t.length>3&&t[0][0]===t[t.length-1][0]&&t[0][1]===t[t.length-1][1]&&t.pop(),t.length<3?null:t});t.forEach((t,n)=>{let r=c[n];if(!r)return;let i=this.oneBuilding(o.as(n),r,t.f,!0);s[n]={id:t.f.id??`${e}:${n}`,props:{...t.f.props,"height (m)":+i.toFixed(1)}}}),this.put(e,{origin:r,side:500,at:[n*500,a*500],mesh:()=>{let e=new ce;return t.forEach((t,n)=>{let r=c[n];r&&this.oneBuilding(e.as(n),r,t.f,!1)}),e.build()},far:o.build(),info:e=>{let t=s[e];return t?{kind:`building`,id:t.id,props:t.props}:null}})}}oneBuilding(e,t,n,r){let i=this.bp,a=n.props,o=i.floor??3.2,l=Wg(a[`building:levels`])??Wg(a.levels)??Wg(a.num_floors),u=Math.max(2.5,i.height?.(n)??Wg(a.height)??Wg(a.h)??Wg(a.est_height)??(l?l*o:null)??i.fallback??6.4),d=bm(t)>0?t:[...t].reverse(),f=Math.abs(bm(d)),p=0,m=0;for(let[e,t]of d)p+=e,m+=t;p/=d.length,m/=d.length,e.on([p,m]);let h=new Tm(Em(p*100,m*100,d.length)),g=String(a.class??a.cls??a.building??a.subtype??``),_=typeof i.look==`function`?i.look:null,v=_?null:i.look,y=_?.(n)??{},b=-(i.sink??1);if((i.house??((e,t,n)=>qm(g,t,n)))(n,f,u)){let t=i.roofs===!0?Ug(a):null,o=i.front?i.front(n)??null:Wg(a.front),s={...v?.house??Wm,...t===`gable`||t===`hip`?{forms:{[t]:1}}:{},...y.wall?{walls:{[y.wall]:1}}:{},...y.roof?{roofs:{[y.roof]:1}}:{},...y.window?{windowColor:y.window}:{}};return Zm(e,{ring:d,height:u,...o===null?{}:{front:{x:Math.sin(o*G),z:-Math.cos(o*G)}}},s,h,r,b,i.windows!==!1),u}let x=v?.block??Gm,S=y.wall??c(h.pick(x.walls),h.next(),.06),C=y.roof??h.pick(x.roofs),w=y.window??x.window,T=i.roofs===!0?Ug(a):typeof i.roofs==`function`?i.roofs(n)??null:null,E=Sm(d),D=(Wg(a[`roof:angle`])??i.pitch??25)*G,O=T&&typeof T==`string`?T:null,k=!T||T===`flat`?0:Math.min(u*.6,Wg(a[`roof:height`])??(O===`dome`?E.b:O===`skillion`?2*E.b*Math.tan(D)*.5:E.b*Math.tan(D))),A=u-k;return e.prism(d,b,A,S,C,s(d.flat(),[],2)),T&&T!==`flat`&&(typeof T==`string`?qg(e,E,A,k,T,C,S):T.into(e,{width:E.a*2,depth:E.b*2,height:k},{at:[E.cx,A,E.cz],turn:Math.atan2(E.uz,E.ux)*180/Math.PI,level:r?2:0})),!r&&i.windows!==!1&&A>o*2&&Kg(e,d,A,o,w),u}};function Kg(e,t,n,r,i){e.material($e.windows);let a=t.length;for(let o=0;o<a;o++){let[s,c]=t[o],[l,u]=t[(o+1)%a],d=Math.hypot(l-s,u-c);if(d<3)continue;let f=(l-s)/d,p=(u-c)/d,m=-p*.06,h=f*.06,g=Math.max(1,Math.floor(d/r)),_=Math.min(1.8,d/g-.8);for(let t=r*.35;t+r*.45<n-.6;t+=r)for(let n=0;n<g;n++){let a=(n+.5)/g*d;e.wall(s+f*(a-_/2)+m,c+p*(a-_/2)+h,s+f*(a+_/2)+m,c+p*(a+_/2)+h,t,t+r*.45,i)}}e.material(0)}function qg(e,t,n,r,i,a,o){let{cx:s,cz:c,ux:l,uz:u,vx:d,vz:f,a:p,b:m}=t,h=n+r,g=[s,n+r*.4,c],_=(e,t,n)=>[s+l*e+d*t,n,c+u*e+f*t],v=(t,n,r,i)=>{let a=[(n[1]-t[1])*(r[2]-t[2])-(n[2]-t[2])*(r[1]-t[1]),(n[2]-t[2])*(r[0]-t[0])-(n[0]-t[0])*(r[2]-t[2]),(n[0]-t[0])*(r[1]-t[1])-(n[1]-t[1])*(r[0]-t[0])],o=[(t[0]+n[0]+r[0])/3-g[0],(t[1]+n[1]+r[1])/3-g[1],(t[2]+n[2]+r[2])/3-g[2]];a[0]*o[0]+a[1]*o[1]+a[2]*o[2]>=0?e.tri(t,n,r,i):e.tri(t,r,n,i)},y=(e,t,n,r,i)=>{v(e,t,n,i),v(e,n,r,i)},b=_(-p,-m,n),x=_(p,-m,n),S=_(p,m,n),C=_(-p,m,n);if(i===`gable`){let e=_(-p,0,h),t=_(p,0,h);y(b,x,t,e,a),y(S,C,e,t,a),v(C,b,e,o),v(x,S,t,o);return}if(i===`hip`){let e=Math.max(0,p-m),t=_(-e,0,h),n=_(e,0,h);y(b,x,n,t,a),y(S,C,t,n,a),v(C,b,t,a),v(x,S,n,a);return}if(i===`pyramidal`){let e=_(0,0,h);v(b,x,e,a),v(x,S,e,a),v(S,C,e,a),v(C,b,e,a);return}if(i===`skillion`){let e=_(-p,-m,h),t=_(p,-m,h);y(e,t,S,C,a),y(b,x,t,e,o),v(C,b,e,o),v(x,S,t,o);return}let w=m;for(let e=0;e<6;e++)for(let t=0;t<16;t++){let i=(e,t)=>{let i=e/6*(Math.PI/2),a=t/16*Math.PI*2,o=w*Math.cos(i);return _(Math.cos(a)*o,Math.sin(a)*o,n+r*Math.sin(i))};e===5?v(i(e,t),i(e,t+1),i(6,0),a):y(i(e,t),i(e,t+1),i(e+1,t+1),i(e+1,t),a)}}const Jg=e=>new Gg(e),Yg=`#cfd0d1`;function Xg(e,t,n,r,i,a){let o=a.eaves,s=Bt(t,n+2*o,r+2*o,{pitch:a.pitch,ridge:`long`,endInset:o,...t===`flat`?{thick:.3}:{}}),c=a.cx??0,l=a.cz??0,u=e=>e.map(e=>[e[0]+c,e[1]+i,e[2]+l]);e.group(`roof`,e=>e.mesh(u(s.slopes),{color:a.roof})),e.mesh(u(s.ends),{color:a.wall}),e.mesh(u(s.under),{color:a.trim,detail:0})}function Zg(e,t,n,r,i,a,o,s,c){let l=Math.max(1,Math.floor(i/3.1)),u=Math.min(1.35,o-1.3),d=Math.atan2(r[0],r[2])*180/Math.PI;if(!(i<2.4||u<.5))for(let f=0;f<a;f++)for(let a=0;a<l;a++){let p=((a+.5)/l-.5)*i;s(f,p)||e.solid(`box`,{size:[Math.min(1.2,i/l-.8),u,.06],at:[t[0]+n[0]*p+r[0]*.02,f*o+.9+u/2,t[2]+n[2]*p+r[2]*.02],turn:d,color:c,material:`glass`,lamps:`windows`,detail:1})}}const Qg=U({id:`house`,kind:n.house,params:{width:k(6,20,10),depth:k(6,20,11),levels:He(1,3,1),floor:k(2.7,3.6,3),roof:u([`gable`,`hip`,`dutch`,`pyramidal`,`skillion`,`flat`],`gable`),pitch:k(8,50,25),eaves:k(0,.8,.45),garage:u([`right`,`left`,`none`],`right`),driveway:k(0,20,6),paved:N(!0),wall:ze(`#f2f2f0`),roofing:ze(`#9b9fa4`),trim:ze(`#d6d7d9`),glass:ze(`#3c4248`),door:ze(`#6a6e73`)},budget:{whole:1600,simple:900},simple:!0,far:(e,t)=>{let n=t.levels*t.floor,r=t.garage===`left`?1:-1;e.solid(`box`,{size:[t.width,n+(t.roof===`flat`?.3:t.width*.18),t.depth],at:[0,(n+(t.roof===`flat`?.3:t.width*.18))/2,0],color:t.wall}),t.garage!==`none`&&e.solid(`box`,{size:[3.8,3,6.4],at:[r*(t.width/2+2.1),1.5,t.depth/2-3.2],color:t.wall})},build:(e,t)=>{let n=t.width,r=t.depth,i=t.levels*t.floor,a=r/2,o=t.garage!==`none`,s=t.garage===`left`?1:-1,c=s*(n/2+.2+1.9),l=o?-s*Math.min(n*.2,n/2-1.5):0;if(e.solid(`box`,{size:[n+.14,.42,r+.14],at:[0,-.09,0],color:t.trim}),e.solid(`box`,{size:[n,i,r],at:[0,i/2,0],color:t.wall}),Xg(e,t.roof,n,r,i,{pitch:t.roof===`skillion`?Math.min(t.pitch,15):t.pitch,eaves:t.eaves,roof:t.roofing,wall:t.wall,trim:t.trim}),e.solid(`box`,{size:[1,2.1,.08],at:[l,1.05,a+.03],color:t.door}),e.solid(`box`,{size:[1.5,.16,.7],at:[l,.08,a+.35],color:t.trim,detail:1}),Zg(e,[0,0,a],[1,0,0],[0,0,1],n,t.levels,t.floor,(e,t)=>e===0&&Math.abs(t-l)<1.4,t.glass),Zg(e,[0,0,-a],[-1,0,0],[0,0,-1],n,t.levels,t.floor,()=>!1,t.glass),Zg(e,[n/2,0,0],[0,0,-1],[1,0,0],r,t.levels,t.floor,()=>!1,t.glass),Zg(e,[-n/2,0,0],[0,0,1],[-1,0,0],r,t.levels,t.floor,()=>!1,t.glass),e.anchor(`entry-1`,[l,0,a+.9],{kind:`entry`,heading:180}),o)e.solid(`box`,{size:[3.8,2.8,6.4],at:[c,1.4,a-3.2],color:t.wall}),e.group(`roof`,e=>e.solid(`box`,{size:[4,.2,6.6],at:[c,2.9,a-3.2],color:t.roofing})),e.solid(`box`,{size:[2.2,2.2,.06],at:[c-s*.4,1.1,a+.02],color:t.trim}),t.paved&&t.driveway>0&&e.solid(`box`,{size:[6.2,.04,t.driveway],at:[c-s*1.2,.02,a+t.driveway/2],color:Yg,detail:1}),e.anchor(`service-1`,[c+s*.9,0,a+.25],{kind:`service`,heading:0}),e.anchor(`parking-1`,[c+s*.9,0,a+3.15],{kind:`parking`,heading:180}),e.anchor(`parking-2`,[c-s*1.9,0,a+3.15],{kind:`parking`,heading:180});else{let r=n/2+1.6;t.paved&&t.driveway>0&&e.solid(`box`,{size:[3,.04,t.driveway+2],at:[r,.02,a+t.driveway/2-1],color:Yg,detail:1}),e.anchor(`service-1`,[n/2-.5,0,a+.25],{kind:`service`,heading:0}),e.anchor(`parking-1`,[r,0,a+3.15],{kind:`parking`,heading:180})}}}),$g=U({id:`office`,kind:n.office,params:{width:k(12,60,24),depth:k(8,30,14),levels:He(1,12,4),floor:k(3,4.5,3.4),roof:u([`flat`,`skillion`,`gable`,`hip`],`flat`),pitch:k(5,35,12),entries:He(2,6,2),carpark:He(1,2,2),bays:He(4,20,9),bay:k(2.4,3.2,2.7),paved:N(!0),wall:ze(`#e6e7e8`),roofing:ze(`#a9adb1`),trim:ze(`#c5c8cb`),glass:ze(`#5d6268`),asphalt:ze(`#64686d`)},budget:{whole:2600,simple:1300},simple:!0,far:(e,t)=>{let n=t.levels*t.floor;e.solid(`box`,{size:[t.width,n+.6,t.depth],at:[0,(n+.6)/2,0],color:t.wall})},build:(e,t)=>{let n=t.width,r=t.depth,i=t.levels*t.floor,a=r/2,o=t.bay,s=t.bays,c=t.carpark;e.solid(`box`,{size:[n,i,r],at:[0,i/2,0],color:t.wall});for(let i=0;i<t.levels;i++){let o=i*t.floor+t.floor*.55,s=t.floor*.5;for(let[c,l,u]of[[n-.8,[0,o,a+.03],0],[n-.8,[0,o,-a-.03],0],[r-.8,[n/2+.03,o,0],90],[r-.8,[-n/2-.03,o,0],90]])i===0&&l[2]>0||e.solid(`box`,{size:[c,s,.06],at:l,turn:u,color:t.glass,material:`glass`,lamps:`windows`,detail:1})}if(e.solid(`box`,{size:[n-.8,t.floor*.75,.06],at:[0,t.floor*.4,a+.03],color:t.glass,material:`glass`,detail:1}),t.roof===`flat`){e.group(`roof`,e=>e.solid(`box`,{size:[n,.2,r],at:[0,i+.1,0],color:t.roofing}));for(let[o,s]of[[[n+.1,.7,.2],[0,i+.35,a-.05]],[[n+.1,.7,.2],[0,i+.35,-a+.05]],[[.2,.7,r],[n/2-.05,i+.35,0]],[[.2,.7,r],[-n/2+.05,i+.35,0]]])e.solid(`box`,{size:o,at:s,color:t.trim})}else Xg(e,t.roof,n,r,i,{pitch:t.pitch,eaves:.4,roof:t.roofing,wall:t.wall,trim:t.trim});let l=Array.from({length:t.entries},(e,r)=>t.entries===1?0:(r/(t.entries-1)-.5)*(n*.6));e.solid(`box`,{size:[n*.6+3,.18,1.8],at:[0,3,a+.9],color:t.trim,detail:1}),l.forEach((t,n)=>{e.solid(`box`,{size:[1.8,2.3,.08],at:[t,1.15,a+.06],color:`#454b51`,material:`glass`}),e.anchor(`entry-${n+1}`,[t,0,a+.9],{kind:`entry`,heading:180})});let u=Array.from({length:s},(e,t)=>(t-(s-1)/2)*o),d=a+3.8,f=a+15.7;if(c>0&&t.paved){let n=a+.6,r=c===2?a+20.5:a+6.6;e.solid(`box`,{size:[s*o+1.5,.04,r-n],at:[0,.02,(n+r)/2],color:t.asphalt,detail:1});for(let t of c===2?[d,f]:[d])for(let n=0;n<=s;n++)e.solid(`box`,{size:[.12,.02,5],at:[(n-s/2)*o,.045,t],color:`#f7f7f5`,detail:1})}let p=0;c>0&&u.forEach((t,n)=>{e.anchor(`parking-${++p}`,[t,0,d],{kind:`parking`,heading:180}),e.anchor(`service-${n+1}`,[t,0,a+.9],{kind:`service`,heading:0})}),c>1&&u.forEach(t=>e.anchor(`parking-${++p}`,[t,0,f],{kind:`parking`,heading:0}))}}),e_={house:Qg,office:$g,make:(e,t={},n=1)=>e===`house`?Qg.make(t,n):$g.make(t,n)},t_={shallow:`#2fb3ad`,deep:`#0b3a57`,clarity:9,speed:6,from:225,glitter:1,waves:16,reach:300},n_=9.81,r_=Math.PI/180,i_=(e,t)=>{let n=Math.imul(e+1,2654435761)^Math.imul(t+7,1597334677);return n=Math.imul(n^n>>>15,2246822519),n^=n>>>13,(n>>>0)/4294967296};function a_(e){let t=Math.max(1,Math.min(16,e.count??16)),n=Math.max(.5,e.speed),r=.877*n_/n,i=[],a=e=>{let t=(e+180)*r_;return[Math.sin(t),-Math.cos(t)]},o=e.swell||null;if(o&&(o.height??1)>0){let t=o.period??11,n=2*Math.PI/t,r=n*n/n_,s=(o.height??1)/2;i.push({dir:a(o.from??e.from),k:r,a:s,omega:n,q:0})}let s=t-i.length,c=s>1?10**(1/(s-1)):1,l=e.seed??1;for(let t=0;t<s;t++){let n=.72*r*c**t,a=n*(c-1||.5),o=.0081*n_*n_/n**5*Math.exp(-1.25*(r/n)**4),u=Math.sqrt(2*o*a),d=(22+40*(t/Math.max(1,s-1)))*r_,f=(e.from+180)*r_+(i_(t,l)-.5)*2*d;i.push({dir:[Math.sin(f),-Math.cos(f)],k:n*n/n_,a:Math.min(u,.14/(n*n/n_)),omega:n,q:0})}for(let e of i)e.q=Math.min(1,.6/(i.length*e.k*e.a+1e-9));return i}const o_=e=>{let t=e.replace(`#`,``),n=e=>parseInt(t.slice(e,e+2),16)/255;return[n(0),n(2),n(4)]},s_={surf:120,runup:12,period:9,foam:1,shelf:.02,cell:2};function c_(e,t,n){let r=[],i=e=>{for(let i=0;i+1<e.length;i++){let a=e[i],o=e[i+1];if(a[0]===o[0]&&a[1]===o[1]||n&&!(a[0]>=n[0]&&a[0]<=n[2]&&a[1]>=n[1]&&a[1]<=n[3]||o[0]>=n[0]&&o[0]<=n[2]&&o[1]>=n[1]&&o[1]<=n[3]))continue;let s=t(a[0],a[1]),c=t(o[0],o[1]);r.push(s[0],s[1],c[0],c[1])}};for(let t of e){let e=t.geometry;if(e){if(e.type===`Polygon`||e.type===`MultiPolygon`)for(let t of Xe(e))t.forEach((e,t)=>{let n=be(e)>0;i(t===0===n?e:[...e].reverse())});else for(let t of z(e,!1))i(t)}}return Float64Array.from(r)}const l_=e=>{let t=111320*Math.cos(e[1]*Math.PI/180);return(n,r)=>[(n-e[0])*t,(r-e[1])*110574]};function u_(e,t,n){let{x0:r,y0:i,cell:a,nx:o,ny:s}=t,c=o*s,l=new Float32Array(c).fill(1/0),u=new Float32Array(c),d=new Int8Array(c),f=new Float32Array(c).fill(1e6),p=0,m=!1;return{out:f,step:(t=1/0)=>{if(m)return!0;let h=Math.min(e.length,p+4*Math.max(1,Math.floor(t)));for(;p<h;p+=4){let t=e[p],c=e[p+1],f=e[p+2],m=e[p+3],h=f-t,g=m-c,_=h*h+g*g,v=Math.sqrt(_),y=Math.max(0,Math.floor((Math.min(t,f)-n-r)/a)),b=Math.min(o-1,Math.ceil((Math.max(t,f)+n-r)/a)),x=Math.max(0,Math.floor((i-Math.max(c,m)-n)/a)),S=Math.min(s-1,Math.ceil((i-Math.min(c,m)+n)/a));for(let e=x;e<=S;e++){let s=i-(e+.5)*a,f=e*o;for(let e=y;e<=b;e++){let i=r+(e+.5)*a,o=Math.max(0,Math.min(1,((i-t)*h+(s-c)*g)/_)),p=i-(t+o*h),m=s-(c+o*g),y=Math.sqrt(p*p+m*m);if(y>n)continue;let b=f+e,x=h*(s-c)-g*(i-t),S=Math.abs(x)/(v*Math.max(y,1e-6));(y<l[b]-1e-4||y<=l[b]+1e-4&&S>u[b])&&(l[b]=y,u[b]=S,d[b]=x>0?-1:1)}}}if(p>=e.length){for(let e=0;e<c;e++)l[e]<=n&&(f[e]=d[e]*l[e]);m=!0}return m}}}function d_(e){return Array.isArray(e)?e.map(e=>e.props?e:{...e,props:e.properties??{}}):e.features()}const f_=1024,p_=1.075,m_=(p_**96-1)/.07499999999999996;var h_=class extends D{kind=`custom`;look;shore;g=null;prog=null;block=null;index=null;u=/* @__PURE__ */ new Float32Array(168);waves=[];wavesKey=``;anchor=NaN;share=0;lastNow=-1;src=null;off=null;coastV=-1;fields=[];job=null;none=!1;seen=!1;eyeH=1/0;constructor(e={}){super({id:`sea`,order:.5,...e}),this.look=e.look===!0||!e.look?{}:e.look,this.shore=e.shore??null}seaLevel(e){return Pe(this.props.level??0,e)}sea(e){return{level:this.seaLevel(e),color:o_(this.look.deep??t_.deep),look:this.pack(e)}}async attach(e){await super.attach(e);let t=this.shore?.coast;t instanceof Te&&(this.src=t,this.off=t.watch(()=>{this.coastV=-1}),t.start({loader:e.loader,error:t=>e.world.error(t)}));let n=e.graphics;if(!n)return;this.g=n,this.block=n.buffer(`uniform`,672,`${this.id} block`);let r=/* @__PURE__ */ new Uint32Array(110592),i=0;for(let e=0;e<96;e++)for(let t=0;t<192;t++){let n=e*193+t;r.set([n,n+193,n+1,n+1,n+193,n+193+1],i),i+=6}this.index=n.buffer(`index`,r,`${this.id} rings`),this.prog=n.program({label:`${this.id} surface`,wgsl:kf,glsl:Mf,blocks:[`Frame`,`Sea`],textures:[{name:`seaField`,kind:`2d`,sample:`unfilterable`},{name:`shadowMap`,kind:`2d`,sample:`float`}],blend:`alpha`,depth:{write:!0,test:`less`},cull:`none`}),await this.prog.ready.catch(e=>{this.failure=e})}detach(){this.off?.(),this.off=null,this.src=null;for(let e of this.fields)e.tex.destroy();for(let e of this.retired)e.destroy();this.fields=[],this.retired=[],this.job=null,this.prog?.destroy(),this.block?.destroy(),this.index?.destroy(),this.dummy?.destroy(),this.prog=null,this.block=this.index=this.dummy=null,super.detach()}dummy=null;retired=[];wantsDisplaced(e){let t=this.look.tier??`auto`;return t===`auto`?e.render.quality===`realistic`&&(e.render.detail??1)>=.75:t===`displaced`}pack(e){let t=this.attached,n=this.g;if(!t||!n||!this.block)return null;for(let e of this.retired)e.destroy();this.retired=[],this.fieldStep();let r=t.camera,i=t.world,a=i.now,o=i.light??null,s=this.look,c=this.u,l=this.lastNow<0?0:Math.max(0,a-this.lastNow);this.lastNow=a;let u=Pe(s.wind?.speed??(o?.wind&&o.wind>0?o.wind:t_.speed),e),d=Pe(s.wind?.from??t_.from,e),f=`${u.toFixed(2)}|${d.toFixed(1)}|${JSON.stringify(s.swell??null)}|${s.waves??t_.waves}`;f!==this.wavesKey&&(this.wavesKey=f,this.waves=a_({speed:u,from:d,swell:s.swell,count:s.waves??t_.waves}));let p=this.wantsDisplaced(e);this.share=p?Math.min(1,this.share+l/600):Math.max(0,this.share-l/600);let m=!p&&this.share===0,h=this.waves.length,g=m&&e.render.quality===`lite`?Math.min(8,h):h,_=Math.min(8,h);Math.abs(r.lat-this.anchor)<2||(this.anchor=r.lat);let v=Math.cos(this.anchor*G),y=Math.cos(r.lat*G),b=v/y,x=J*v*r.lon*G,S=-J*v*q(Math.max(-85.0511,Math.min(85.0511,r.lat))),C=s.clock?s.clock(e):a/1e3,w=Math.PI*2,T=0;for(let e=0;e<16;e++){let t=this.waves[e];if(!t){c.fill(0,40+e*4,44+e*4),c.fill(0,104+e*4,108+e*4);continue}e>=g&&(T+=.5*(t.k*t.a)**2);let n=t.k*(t.dir[0]*x+t.dir[1]*S)-t.omega*C;c.set([t.dir[0],t.dir[1],t.k,t.a],40+e*4),c.set([n-w*Math.floor(n/w),t.q,0,0],104+e*4)}let E=i.exaggeration,D=this.seaLevel(e)*E,O=this.shore,k=o_(s.shallow??t_.shallow),A=o_(s.deep??t_.deep),j=O?Pe(O.period??s_.period,e):0,M=Math.max(1,r.dist*Math.sin(r.pitch));this.eyeH=M,c.set([1,this.share,_,g],0),c.set([s.glitter??t_.glitter,b,C%36e3,D],4),c.set([...k,s.clarity??t_.clarity],8),c.set([...A,O?.shelf??s_.shelf],12);let N=O?this.fieldFor(r,M):null;if(N){let e=y/Math.cos(N.lat0*G),t=J*y;c.set([t*G*Y(N.lon0-r.lon)+N.x0*e,-t*(q(N.lat0)-q(Math.max(-85.0511,Math.min(85.0511,r.lat))))-N.y0*e,N.cell*e,1],16)}else c.set([0,0,1,0],16);let P=.3;for(let e=0;e<_;e++)P+=this.waves[e]?.a??0;c.set([f_,f_,N?.band??0,P+ +!!O],20),c.set(O?[O.surf??s_.surf,O.runup??s_.runup,j,O.foam??s_.foam]:[s_.surf,0,0,0],24);let F=Math.max(300,Math.min(25e3,M*(s.reach??t_.reach))),I=F/m_,L=I*8,R=x+r.eye[0]*b,z=S+r.eye[2]*b,B=Math.round(R/L)*L,V=Math.round(z/L)*L;return c.set([I,p_,96,F],28),c.set([(B-x)/b,(V-S)/b,r.dist*6,r.dist*40],32),c.set([T,.003+.00512*u,0,0],36),this.block.write(c),this.seen=M<3e4&&this.waterInView(r,D/Math.max(E,1e-6),N),{block:this.block,field:N?.tex??(this.dummy??=n.texture({label:`${this.id} no field`,width:1,height:1,format:`r32f`,filter:`nearest`}))}}waterInView(e,t,n){let r=this.attached.world,i=e.width,a=e.height;for(let[n,o]of[[.5,.5],[.1,.35],[.9,.35],[.5,.15],[.1,.9],[.9,.9],[.5,.75]]){let s=e.lonLatAt(n*i,o*a);if(!s)continue;let c=r.heightAt(s[0],s[1]);if(Number.isFinite(c)&&c<t+.5)return!0}return!!n&&n.key!==``}fieldFor(e,t){let n=this.shore,r=n.cell??s_.cell,i=this.src?this.src.version:0;if(i!==this.coastV){this.coastV=i;for(let e of this.fields)this.retired.push(e.tex);this.fields=[],this.job=null,this.none=!1}let a=Math.max(0,Math.ceil(Math.log2(Math.max(1,t*10/(r*f_*.25))))),o=this.fields[0];if(o){let e=Math.round(Math.log2(o.cell/r));Math.abs(Math.log2(Math.max(1,t*10/(r*f_*.25)))-e)<.5+(a>e?.5:0)&&(a=e)}let s=r*2**a,c=s*f_/2,l=Math.max((n.surf??s_.surf)*1.4,(n.runup??s_.runup)*2+4);if(l<s*3){if(this.fields.length){for(let e of this.fields)this.retired.push(e.tex);this.fields=[]}return this.job=null,null}let u=e.lat,d=e.lon,f=J*q(u),p=c/2;if(!(o&&o.cell===s&&Math.abs(6378137*G*Y(d-o.lon0)*Math.cos(o.lat0*G))<p&&Math.abs((f-6378137*q(o.lat0))*Math.cos(o.lat0*G))<p)&&!this.job){let e=Math.cos(u*G),t=Math.round(d*G*J*e/p)*p/(J*e*G),r=Vt(Math.round(f*e/p)*p/(J*e)),i=`${t.toFixed(6)}|${r.toFixed(6)}|${s}`;if(i!==o?.key){let e=J*Math.cos(r*G),a=(n,i)=>[e*G*Y(n-t),e*(q(i)-q(r))],u=c*1.3/(J*Math.cos(r*G)*G),d=c*1.3/110574,f=[t-u,r-d,t+u,r+d],p=c_(d_(n.coast),a,f);if(!p.length){if(this.none=!0,o){for(let e of this.fields)this.retired.push(e.tex);this.fields=[]}return null}this.none=!1,this.job={key:i,lon0:t,lat0:r,x0:-c,y0:c,cell:s,band:l,f:u_(p,{x0:-c,y0:c,cell:s,nx:f_,ny:f_},l)}}}return this.fields[0]??null}fieldStep(){let e=this.job,t=this.g;if(!e||!t)return;let n=performance.now();for(;!e.f.step(6);)if(performance.now()-n>3)return;let r=t.texture({label:`${this.id} coast field`,width:f_,height:f_,format:`r32f`,filter:`nearest`});r.write(e.f.out);for(let e of this.fields)this.retired.push(e.tex);this.fields=[{tex:r,lon0:e.lon0,lat0:e.lat0,x0:e.x0,y0:e.y0,cell:e.cell,band:e.band,key:e.key}],this.job=null,(this.attached?.world)?.invalidate?.()}frame(e){let t=e.sea,n=e.camera;if(!this.prog||!this.block||!this.index||this.failure||!t?.look||this.share<=0||n.morph<.99||this.eyeH>5e3)return[];let r=t.look.field??this.dummy;return[{program:this.prog,blocks:[e.frame,this.block],textures:[r,e.shadowMap],index:this.index,count:110592}]}wantsFrames(e){return this.seen?this.eyeH<1500?1/0:this.eyeH<6e3?30:10:0}animating(e){return this.wantsFrames(e)>0}status(){let e=[];this.src&&e.push(...this.src.status().pending),this.job&&e.push(`${this.id} coast field`);let t=[...this.src?.status().failed??[],...this.failure?[{what:this.id,error:this.failure.message}]:[]];return{ready:!e.length&&!t.length,pending:e,failed:t,missing:[]}}attribution(){return this.src?.attribution?[this.src.attribution]:[]}};const g_=(e={})=>new h_(e),__=`#c9ced4`,v_=`#0f1b26`,y_=`#f4f5f2`,b_=`#18191b`,x_=U({id:`charger`,kind:n.charger,params:{ports:He(1,8,4),power:u([`7`,`22`,`50`,`150`],`50`),canopy:N(!0),mount:u([`pedestal`,`wall`],`pedestal`),bay:k(2.3,3.4,2.7),connector:u([`ccs2`,`type2`,`chademo`,`ccs1`,`j1772`,`gbt`],`ccs2`),brand:ze(`#8d9298`),body:ze(y_),canopyColor:ze(`#e9ecef`),marking:ze(`#c4c8cc`),sign:N(!0),marked:N(!0)},budget:{whole:3600,simple:1500},simple:!0,far:`box`,build:(e,t)=>{let n=t.ports,r=t.bay,i=Number(t.power)>=50,a=t.mount===`wall`,o=(t,n,r={})=>e.solid(`box`,{size:t,at:n,...r}),s=Array.from({length:n},(e,t)=>(t-(n-1)/2)*r);if(a){for(let[n,i]of s.entries()){if(o([.12,1.25,.12],[i,.62,-.18],{color:__,material:`metal`}),o([.38,.48,.16],[i,1.3,-.08],{color:t.body,round:.03}),o([.2,.14,.02],[i,1.36,.01],{color:v_,material:`glass`,detail:1}),o([.08,.05,.03],[i,1.5,.01],{color:`#34d399`,material:`glass`}),o([.04,.5,.04],[i+.15,1,0],{color:b_,detail:0}),o([.12,.04,.12],[i+.15,.76,.03],{color:b_,detail:0}),t.marked){o([r-.2,.02,5],[i,.01,2.9],{color:t.marking,detail:1});for(let e of[-1,1])o([.1,.025,5],[i+e*r/2,.012,2.9],{color:y_,detail:1})}e.anchor(`light-${n+1}`,[i,1.5,.03],{kind:`light`}),e.anchor(`bay-${n+1}`,[i,0,2.9],{kind:`parking`,heading:180})}return}let c=5.4,l=n*r;for(let[n,i]of s.entries())t.marked&&o([r-.2,.02,c],[i,.01,6.4/2],{color:t.marking,detail:1}),e.anchor(`bay-${n+1}`,[i,0,3.2+.2],{kind:`parking`,heading:180});if(t.marked)for(let e=0;e<=n;e++)o([.12,.025,c],[(e-n/2)*r,.012,6.4/2],{color:y_,detail:1});o([l+.6,.15,1.1],[0,.075,-.15],{color:`#b7bbbf`});let u=i?1.9:1.4;for(let[n,r]of s.entries())o([.62,u,.42],[r,.15+u/2,-.15],{color:t.body,round:.04}),o([.64,.16,.44],[r,.15+u*.18,-.15],{color:t.brand}),o([.44,i?.42:.3,.02],[r,.15+u*.72,.07],{color:v_,material:`glass`}),o([.3,.06,.06],[r,.15+u+.03,0],{color:`#34d399`,material:`glass`}),e.anchor(`light-${n+1}`,[r,.15+u+.06,0],{kind:`light`}),o([.05,u*.5,.05],[r+.36,.15+u*.42,-.02],{color:b_,detail:0}),o([.05,.05,.3],[r+.36,.15+u*.17,.12],{color:b_,detail:0}),o([.1,.14,.08],[r+.36,.15+u*.67,.04],{color:`#23272c`,detail:0});if(t.canopy){let n=4.6,r=l+1.2;for(let t of[-1,1])for(let i of[-1.0999999999999999,5.000000000000001])e.solid(`cylinder`,{radius:.14,size:n,at:[t*(r/2-.3),n/2,i],color:__,material:`metal`,sides:10});o([r,.32,6.700000000000001],[0,4.76,3.900000000000001/2],{color:t.canopyColor}),o([r+.04,.42,.06],[0,4.72,5.3100000000000005],{color:t.brand}),o([r+.04,.42,.06],[0,4.72,-1.41],{color:t.brand,detail:1});for(let e of s)o([.9,.04,.2],[e,4.58,6.4/2],{color:`#fff7e0`,lamps:`windows`,detail:1})}if(t.sign){let n=l/2+1.4,r=6.2;o([.22,r,.22],[n,r/2,.2],{color:__,material:`metal`}),o([1.3,1.3,.16],[n,6.7,.2],{color:t.brand,round:.05});for(let r of[1,-1]){let i=.2+r*.09;e.solid(`cylinder`,{radius:.36,size:.02,axis:`z`,at:[n,6.75,i],color:y_,sides:18,detail:1});for(let e of[-.12,.12])o([.07,.18,.03],[n+e,6.8,i+r*.01],{color:t.brand,detail:1});o([.08,.22,.03],[n,6.5,i+r*.01],{color:y_,detail:1})}}o([l+3.6,.01,8.4],[.6,.004,6.4/2-.6],{color:`#3b3e42`,detail:1})}}),S_={make:(e={},t=1)=>x_.make(e,t),station:(e=4)=>x_.make({ports:e,power:`50`,canopy:!0,mount:`pedestal`}),wall:(e=1,t=!0)=>x_.make({ports:e,power:`7`,canopy:!1,mount:`wall`,sign:!1,marked:t}),shape:x_},C_=`${mt}
layout(std140) uniform U { vec4 a; vec4 b; vec4 c; } u;
`,w_={vertex:`${C_}
layout(location = 0) in vec4 s;
out float v_alpha;
float wrapIn(float v, float size) {
  return v - size * floor(v / size) - size * 0.5;
}
void main() {
  float side = u.a.x;
  float h = u.a.y;
  float t = u.c.z;
  float speed = 9.0 * s.w;
  float x = wrapIn(s.x * side - u.c.x, side);
  float z = wrapIn(s.z * side - u.c.y, side);
  float y = wrapIn(s.y * h - t * speed, h);
  vec3 top = frame.eye.xyz + vec3(x, y, z);
  vec3 fall = vec3(u.a.w * 0.06, -1.0, 0.0) * (0.06 * speed);
  int vid = gl_VertexID;
  int k = vid;
  if (vid == 3) {
    k = 2;
  } else if (vid == 4) {
    k = 1;
  } else if (vid == 5) {
    k = 3;
  }
  float end = float(k % 2);
  float side2 = float(k / 2) * 2.0 - 1.0;
  vec3 p = top + fall * end;
  vec4 c0 = frame.viewProj * vec4(top, 1.0);
  vec4 c1 = frame.viewProj * vec4(top + fall, 1.0);
  vec2 dir = normalize((c1.xy / max(c1.w, 0.001) - c0.xy / max(c0.w, 0.001)) * frame.size.xy + vec2(0.0001, 0.0));
  vec2 across = vec2(-dir.y, dir.x) / frame.size.xy * frame.size.z * 1.2;
  vec4 c = frame.viewProj * vec4(p, 1.0);
  c = vec4(c.xy + across * side2 * c.w, c.z, c.w);
  gl_Position = clipOut(c);
  float dist = length(vec2(x, z));
  v_alpha = u.b.w * (1.0 - smoothstep(side * 0.25, side * 0.5, dist)) * (0.35 + 0.65 * u.a.z);
}
`,fragment:`${C_}
in float v_alpha;
out vec4 outColor;
void main() {
  if (v_alpha <= 0.002) {
    discard;
  }
  outColor = vec4(themed(u.b.rgb, 0.0), v_alpha);
}
`},T_=`${nn}
// a: (the box's side m, its height m, how hard it rains 0..1, the wind m/s) · b: (colour rgb, opacity) ·
// c: (where the eye is in metres of a frame fixed to the ground, x, z; the rain's clock s; 0)
struct U { a: vec4f, b: vec4f, c: vec4f }
@group(0) @binding(1) var<uniform> u: U;
struct I { @location(0) s: vec4f }
struct O { @builtin(position) pos: vec4f, @location(0) alpha: f32 }
fn wrapIn(v: f32, size: f32) -> f32 {
  return v - size * floor(v / size) - size * 0.5;
}
@vertex fn vs(i: I, @builtin(vertex_index) vid: u32) -> O {
  let side = u.a.x;
  let h = u.a.y;
  let t = u.c.z;
  let speed = 9.0 * i.s.w;
  // where the streak is: its place in the box, the box round the eye (fixed to the ground as the eye moves)
  let x = wrapIn(i.s.x * side - u.c.x, side);
  let z = wrapIn(i.s.z * side - u.c.y, side);
  let y = wrapIn(i.s.y * h - t * speed, h);
  let top = frame.eye.xyz + vec3f(x, y, z);
  let fall = vec3f(u.a.w * 0.06, -1.0, 0.0) * (0.06 * speed);
  // two triangles: corners 0 1 2, 2 1 3 (end along the fall, side across it)
  var k = vid;
  if (vid == 3u) {
    k = 2u;
  } else if (vid == 4u) {
    k = 1u;
  } else if (vid == 5u) {
    k = 3u;
  }
  let end = f32(k % 2u);
  let side2 = f32(k / 2u) * 2.0 - 1.0;
  let p = top + fall * end;
  var o: O;
  let c0 = frame.viewProj * vec4f(top, 1.0);
  let c1 = frame.viewProj * vec4f(top + fall, 1.0);
  let dir = normalize((c1.xy / max(c1.w, 0.001) - c0.xy / max(c0.w, 0.001)) * frame.size.xy + vec2f(0.0001, 0.0));
  let across = vec2f(-dir.y, dir.x) / frame.size.xy * frame.size.z * 1.2;
  var c = frame.viewProj * vec4f(p, 1.0);
  c = vec4f(c.xy + across * side2 * c.w, c.z, c.w);
  o.pos = clipOut(c);
  let dist = length(vec2f(x, z));
  o.alpha = u.b.w * (1.0 - smoothstep(side * 0.25, side * 0.5, dist)) * (0.35 + 0.65 * u.a.z);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  if (i.alpha <= 0.002) {
    discard;
  }
  return vec4f(themed(u.b.rgb, 0.0), i.alpha);
}
`;var E_=class extends D{kind=`custom`;prog=null;ubuf=null;ibuf=null;n=0;u=/* @__PURE__ */ new Float32Array(12);amountNow=0;constructor(e={}){super({id:`rain`,order:4.6,...e})}async attach(e){await super.attach(e);let t=e.graphics;if(!t)return;this.n=this.props.count??(e.profile?.quality===`lite`?2500:6e3);let n=new Float32Array(this.n*4),r=12345,i=()=>(r=r*1664525+1013904223>>>0,r/4294967296);for(let e=0;e<this.n;e++)n.set([i(),i(),i(),.7+i()*.6],e*4);this.ibuf=t.buffer(`instance`,n,`rain`),this.ubuf=t.buffer(`uniform`,48,`rain block`),this.prog=t.program({label:`rain`,wgsl:T_,glsl:w_,blocks:[`Frame`,`U`],instance:{stride:4,attributes:[{name:`s`,location:0,size:4,offset:0}]},blend:`alpha`,depth:{write:!1,test:`less`},cull:`none`}),await this.prog.ready}detach(){this.prog?.destroy(),this.ubuf?.destroy(),this.ibuf?.destroy(),this.prog=null,this.ubuf=this.ibuf=null,super.detach()}frame(e){if(!this.prog||!this.ubuf||!this.ibuf)return[];let t=this.props,n=e.world.light??null,r=e.camera,i=typeof t.amount==`function`?t.amount(e.s):t.amount??n?.rain??0,a=r.dist*Math.sin(r.pitch);if(this.amountNow=r.morph<.98||a>(t.ceiling??3e3)?0:Math.max(0,Math.min(1,i)),this.amountNow<.02)return[];let[o,s]=t.box??[70,40],c=n?.haze??[.7,.75,.8],l=n?.daylight??1,u=n?.flash??0,d=t.color??[(c[0]*.5+.4)*(.4+.6*l+u),(c[1]*.5+.42)*(.4+.6*l+u),(c[2]*.5+.45)*(.4+.6*l+u)],f=J*Math.cos(r.lat*G),p=r.eye,m=((f*r.lon*G+p[0])%o+o)%o,h=((-f*Math.log(Math.tan(Math.PI/4+r.lat*G/2))+p[2])%o+o)%o;return this.u.set([o,s,this.amountNow,n?.wind??8],0),this.u.set([d[0],d[1],d[2],t.opacity??.55],4),this.u.set([m,h,t.at??e.now/1e3,0],8),this.ubuf.write(this.u),[{program:this.prog,blocks:[e.frame,this.ubuf],instances:this.ibuf,count:6,instanceCount:Math.max(1,Math.floor(this.n*this.amountNow))}]}animating(){return this.amountNow>=.02&&this.props.at===void 0}};const D_=(e={})=>new E_(e),O_={current:(e,t={})=>k_(e,t),url(e){let t=e.today??(/* @__PURE__ */ new Date()).toISOString().slice(0,10),n=e.date<t;return{url:`${n?`https://archive-api.open-meteo.com/v1/archive`:`https://api.open-meteo.com/v1/forecast`}?latitude=${e.lat}&longitude=${e.lon}&start_date=${e.date}&end_date=${e.date}&hourly=temperature_2m,relative_humidity_2m,wind_speed_10m,cloud_cover,direct_radiation,diffuse_radiation,precipitation&wind_speed_unit=ms&timezone=${encodeURIComponent(e.zone??`GMT`)}`,source:n?`Open-Meteo Historical (ERA5)`:`Open-Meteo Forecast`}},async day(e){let{url:t,source:n}=O_.url(e),r=(e.loader?await e.loader.get(t,{kind:`json`,priority:`now`}):await(await fetch(t)).json())?.hourly;if(!r?.time)throw Error(`no weather for ${e.date}`);let i=(e,t)=>Number(r[e]?.[t]??0),a=r.time.map((e,t)=>({hour:Number(String(e).slice(11,13)),ta:i(`temperature_2m`,t),rh:i(`relative_humidity_2m`,t),u10:i(`wind_speed_10m`,t),cloud:i(`cloud_cover`,t)/100,dirH:i(`direct_radiation`,t),difH:i(`diffuse_radiation`,t),precip:i(`precipitation`,t)}));return{date:e.date,source:n,hours:a}}};async function k_(e,t={}){let n=`https://api.open-meteo.com/v1/forecast?latitude=${e.map(e=>e[1].toFixed(4)).join(`,`)}&longitude=${e.map(e=>e[0].toFixed(4)).join(`,`)}&current=wind_speed_10m,precipitation,visibility,cloud_cover&wind_speed_unit=ms`,r=t.loader?await t.loader.get(n,{kind:`json`,priority:`now`}):await(await fetch(n)).json();return(Array.isArray(r)?r:[r]).map((t,n)=>({geometry:{type:`Point`,coordinates:e[n]},props:{wind:t.current?.wind_speed_10m??null,rain:t.current?.precipitation??null,visibility:t.current?.visibility??null,cover:t.current?.cloud_cover===void 0?null:t.current.cloud_cover/100,source:`Open-Meteo current`}}))}const A_={hour:15,ta:41.3,rh:15,u10:4.9,cloud:0,dirH:866,difH:111,precip:0},j_=110574;function M_(e,t){let n=t.cell??2,r=Math.ceil(t.size/n),i=r,a=r*i,o=t.changes??{},s=e[0]?.ctx.origin??t.centre,c=111320*Math.cos(s[1]*G),l=(t.centre[0]-s[0])*c-t.size/2,u=-(t.centre[1]-s[1])*j_-t.size/2,d=Cf.materials([`drygrass`,`grass`,`asphalt`,`coolpave`,`concrete`,`roof`,`water`]),f={dry:0,grass:1,asphalt:2,cool:3,concrete:4,roof:5,water:6},p=o.wateredGrass?f.grass:f.dry,m=new Uint8Array(a).fill(p),h=new Float32Array(a),g=new Float32Array(a),_=new Float32Array(a),v=new Float32Array(a),y=new Uint8Array(a),b=new Uint8Array(a),x=new Uint8Array(a),S=[],C=/* @__PURE__ */ new Map,w=(e,t,n)=>{n>=x[e]&&(x[e]=n,m[e]=t)},T=(e,t,n)=>[e.ctx.at[0]+t-l,e.ctx.at[1]+n-u],E=(e,t,n)=>{let r=[],i=e[t],a=e[t+1];r.push(T(n,i/10,a/10));for(let o=t+2;o+1<e.length;o+=2)i+=e[o],a+=e[o+1],r.push(T(n,i/10,a/10));return r},D=(e,t)=>{let a=1/0,o=-1/0,s=1/0,c=-1/0;for(let[t,n]of e)a=Math.min(a,t),o=Math.max(o,t),s=Math.min(s,n),c=Math.max(c,n);for(let l=Math.max(0,Math.floor(s/n));l<=Math.min(i-1,Math.floor(c/n));l++)for(let i=Math.max(0,Math.floor(a/n));i<=Math.min(r-1,Math.floor(o/n));i++){let a=(i+.5)*n,o=(l+.5)*n,s=!1;for(let t=0,n=e.length-1;t<e.length;n=t++){let[r,i]=e[t],[c,l]=e[n];i>o!=l>o&&a<(c-r)*(o-i)/(l-i)+r&&(s=!s)}s&&t(l*r+i)}},O=(e,t,a,o)=>{for(let s=Math.max(0,Math.floor((t-a)/n));s<=Math.min(i-1,Math.floor((t+a)/n));s++)for(let i=Math.max(0,Math.floor((e-a)/n));i<=Math.min(r-1,Math.floor((e+a)/n));i++)Math.hypot((i+.5)*n-e,(s+.5)*n-t)<=a&&o(s*r+i)},k=(e,n)=>e.some(([e,r])=>e>-n&&r>-n&&e<t.size+n&&r<t.size+n);for(let a of e){let e=e=>a.tile.s?.[e]??``;for(let t of a.tile.parks){let n=E(t,3,a);if(!k(n,50))continue;let r=e(t[1]),i=/natural/i.test(r)?f.dry:f.grass;D(n,e=>w(e,i,1))}for(let e of a.tile.water){let t=E(e,3,a);k(t,50)&&D(t,e=>w(e,f.water,2))}for(let s of a.tile.roads){let c=s[0],l=s[1]/10,u=e(s[2]),d=E(s,(a.tile.v??1)>=2?4:3,a);if(!k(d,60)||c>=6)continue;let p=l/2,m=Th({cls:c,width:l},Sh),h=m.length?m[m.length-1].to:p,x=0;u&&c>=2&&(x=C.get(u)??0,!x&&S.length<254&&(S.push(u),x=S.length,C.set(u,x)));for(let e=0;e+1<d.length;e++){let[a,s]=d[e],[l,u]=d[e+1],S=l-a,C=u-s,T=S*S+C*C||1,E=Math.min(a,l)-h,D=Math.max(a,l)+h,k=Math.min(s,u)-h,A=Math.max(s,u)+h;for(let e=Math.max(0,Math.floor(k/n));e<=Math.min(i-1,Math.floor(A/n));e++)for(let t=Math.max(0,Math.floor(E/n));t<=Math.min(r-1,Math.floor(D/n));t++){let i=(t+.5)*n,c=(e+.5)*n,l=Math.max(0,Math.min(1,((i-a)*S+(c-s)*C)/T)),u=Math.hypot(i-a-S*l,c-s-C*l),d=e*r+t;if(u<=p)w(d,o.coolRoads?f.cool:f.asphalt,6),x&&(y[d]=x);else for(let e of m)if(u>e.from&&u<=e.to){let t=e.color===Sh.stripColor;w(d,t?o.wateredGrass?f.grass:f.dry:f.concrete,t?4:5),x&&(y[d]=x),b[d]=1}}if(o.streetTrees&&c>=4&&c<=5&&m.length){let e=[...Yh(t.trees).values()][0].transmissivity,n=typeof o.streetTrees==`object`?o.streetTrees:{},r=n.every??8,i=Math.sqrt(T),c=p+Sh.kerb+Sh.strip/2;for(let t=r/2;t<i;t+=r)for(let r of[-1,1]){let o=a+S/i*t-C/i*c*r,l=s+C/i*t+S/i*c*r,u=n.height??10;O(o,l,n.radius??4.2,t=>{g[t]=Math.max(g[t],u),_[t]=_[t]?Math.min(_[t],3):3,v[t]=v[t]?Math.min(v[t],e):e})}}}}for(let e of a.tile.buildings){let t=E(e,3,a);if(!k(t,10))continue;let n=e[0]/10;D(t,e=>{h[e]=n,w(e,f.roof,9),b[e]=0})}let s=a.tile.trees;for(let e=0;e+3<s.length;e+=4){let[n,r]=T(a,s[e]/10,s[e+1]/10);if(n<-20||r<-20||n>t.size+20||r>t.size+20)continue;let i=Zh({x:0,z:0,height:s[e+2]/10,radius:s[e+3]/10,id:0,seed:Em(a.ctx.ix,a.ctx.iz,e)},t.trees);O(n,r,i.radius,e=>{h[e]>0||(g[e]=Math.max(g[e],i.top),_[e]=_[e]?Math.min(_[e],Math.max(1,i.base)):Math.max(1,i.base),v[e]=v[e]?Math.min(v[e],i.transmissivity):i.transmissivity)})}}return{w:r,h:i,cell:n,mat:m,materials:d,bH:h,cTop:g,cBase:_,cTrans:v,region:y,walk:b,streetNames:S,regions:S.map(e=>({id:e,width:28})),lonLat:(e,t)=>[s[0]+(l+e)/c,s[1]-(u+t)/j_],xzOf:(e,t)=>[(e-s[0])*c-l,-(t-s[1])*j_-u]}}function N_(e,t,n){let r=Gc.field({id:n.id??`heat`,ramp:n.ramp??mf,domain:n.domain,cell:2,alpha:n.alpha??.9,attribution:`heat model (aslnut), weather: Open-Meteo`,value:(n,r)=>{let[i,a]=e.xzOf(n,r),o=Math.floor(i/e.cell),s=Math.floor(a/e.cell);if(o<0||s<0||o>=e.w||s>=e.h)return null;let c=t[s*e.w+o];return Number.isFinite(c)?c:null}}),i=e.lonLat(0,e.h*e.cell),a=e.lonLat(e.w*e.cell,0);return{...r,where:[i[0],i[1],a[0],a[1]],minZoom:14,version:n.version??0}}function P_(e,t){let{id:n,centre:r,size:i,cell:a,changes:o,trees:s,...c}=t;return yf({...c,id:n??`heat`,rasters:M_(e,{centre:r,size:i??360,cell:a??2,changes:o,trees:s})})}const F_=Math.PI/180;function I_(e){let t=0;for(let n of Xe(e.geometry))n.forEach((e,n)=>{let r=111.32*Math.cos((e[0]?.[1]??0)*F_),i=0;for(let t=0,n=e.length-1;t<e.length;n=t++)i+=(e[n][0]*r-e[t][0]*r)*(e[n][1]+e[t][1])*110.574;t+=(n===0?1:-1)*Math.abs(i/2)});return t}var L_=class extends at{opt;seen=``;dead=!1;offs=[];constructor(e){if(super([],{id:`id`}),!e?.areas||typeof e.areas.features!=`function`)throw new K(`E-DATA-001`,`stats needs areas: a source of polygons`,{fix:`stats({ areas: data.geojson('/suburbs.geojson'), of: parks, count: true })`});this.opt=e;let t=()=>this.run();this.offs.push(e.areas.watch(t)),e.of&&this.offs.push(e.of.watch(t)),this.run()}async load(e){await Promise.all([this.opt.areas.start(e),this.opt.of?.start(e)]),this.run()}get options(){return this.opt}setOptions(e){for(let e of this.offs)e();this.offs=[],this.opt={...this.opt,...e};let t=()=>this.run();return this.offs.push(this.opt.areas.watch(t)),this.opt.of&&this.offs.push(this.opt.of.watch(t)),this.seen=``,this.run(),this}ranked(e=`desc`){return[...this.rows()].sort((t,n)=>t.value===null?1:n.value===null?-1:e===`desc`?n.value-t.value:t.value-n.value)}get total(){return this.rows().reduce((e,t)=>e+(t.raw??0),0)}dispose(){this.dead=!0;for(let e of this.offs)e();this.offs=[]}run(){if(this.dead)return;let e=this.opt,t=e.areas.features(),n=e.of?.features()??[],r=`${e.areas.version}/${e.of?.version??-1}`;if(r===this.seen&&this.size)return;if(this.seen=r,!t.length){this.size&&this.set([]);return}let i=t.map(e=>O(e.geometry)),a=new Float64Array(t.length),o=new Float64Array(t.length),s=new Float64Array(t.length),c=new Float64Array(t.length),l=ir(),u=(e,t)=>e===void 0?0:Number(typeof e==`function`?e(t):t.props[e]);for(let r of n){if(e.filter&&!pt(e.filter,r,l))continue;let n=H(r.geometry);if(n)for(let l=0;l<t.length;l++){let d=i[l];if(!(!d||n[0]<d[0]||n[0]>d[2]||n[1]<d[1]||n[1]>d[3])&&dn(t[l].geometry,n)){if(a[l]++,e.sum!==void 0){let t=u(e.sum,r);Number.isFinite(t)&&(o[l]+=t)}if(e.mean!==void 0){let t=u(e.mean,r);Number.isFinite(t)&&(s[l]+=t,c[l]++)}break}}}let d=t.map((t,n)=>{let r=I_(t),i={count:a[n],sum:o[n],mean:c[n]?s[n]/c[n]:null},l;if(e.value!==void 0){let n=typeof e.value==`function`?e.value(t,i):t.props[e.value];l=n==null||n===``?null:Number(n)}else l=e.sum===void 0?e.mean===void 0?i.count:i.mean:i.sum;l!==null&&!Number.isFinite(l)&&(l=null);let u=e.per??`none`,d=1;if(u===`km2`)d=r;else if(typeof u==`function`)d=u(t);else if(u!==`none`){let e=Number(t.props[u]);d=Number.isFinite(e)?e:null}let f=l===null||d===null||!(d>0)?null:l/d*(e.scale??1);return{...t.props,id:t.id,name:e.name?e.name(t):String(t.props.name??t.id),value:f,raw:l,count:i.count,area_km2:+r.toFixed(3),rank:null,share:null}}),f=d.reduce((e,t)=>e+(t.raw??0),0);[...d].filter(e=>e.value!==null).sort((e,t)=>t.value-e.value).forEach((e,t)=>{e.rank=t+1});for(let e of d)e.share=f>0&&e.raw!==null?e.raw/f:null;this.set(d)}};const R_=e=>new L_(e),z_=e=>!!e&&typeof e==`object`&&typeof e.show==`function`&&typeof e.shown==`function`&&!(`frame`in e),B_=e=>e.replace(/[&<>"]/g,e=>({"&":`&amp;`,"<":`&lt;`,">":`&gt;`,'"':`&quot;`})[e]),V_=e=>{if(typeof e!=`string`&&!Array.isArray(e))return null;try{let t=L(e);return`rgba(${Math.round(t[0]*255)},${Math.round(t[1]*255)},${Math.round(t[2]*255)},${Math.max(.35,t[3]).toFixed(2)})`}catch{return null}},H_=[`ground`,`over`,`above`],U_=[`#e4572e`,`#17bebb`,`#ffc914`,`#76b041`,`#9c6ade`,`#2e86ab`,`#f18f01`,`#c73e1d`],W_=e=>e?/Point/i.test(e)?`point`:/Line|Curve/i.test(e)?`line`:/Polygon|Surface/i.test(e)?`area`:null:null;function G_(e,t={}){let n=t.title??`Layers`,r=t.fade??0,i=t=>{if(typeof t!=`string`)return e.list.includes(t)?t:null;try{return e.element(t)}catch{return null}},a=e=>typeof e==`string`?e:e.id,o=t.add?t.add===!0?{}:t.add:null,s=o?.group??Object.keys(t.groups??{})[0]??`Data`,c=[],l=()=>{let n=/* @__PURE__ */ new Set,r=[],o={...t.groups??{}};c.length&&(o[s]=[...o[s]??[],...c]);for(let[e,t]of Object.entries(o)){let o=[];for(let e of t)if(n.add(a(e)),z_(e))o.push(e);else{let t=i(e);t&&o.push(t)}r.push({name:e,entries:o})}let l=(t.of?t.of.map(i).filter(e=>!!e):e.list.filter(e=>!!e.props.label)).filter(e=>!n.has(e.id));l.length&&r.push({name:t.groups||c.length?`Other`:``,entries:l});let u=new Map(e.list.map((e,t)=>[e,t]));for(let e of r){let t=e.entries.filter(e=>!z_(e)).sort((e,t)=>(u.get(t)??0)-(u.get(e)??0)),n=0;e.entries=e.entries.map(e=>z_(e)?e:t[n++])}return r},u=e=>{let n=t.legends?.[e.id],r=e.props,i=n??r.legend??(typeof e.legendImage==`string`?{kind:`image`,items:[],image:e.legendImage}:void 0);if(i)return{legend:me(i)?i.legend({format:t.format}):i,swatch:null};let a=r.color;if(me(a))try{return{legend:a.legend({format:t.format}),swatch:null}}catch{return{legend:null,swatch:null}}return{legend:null,swatch:typeof a==`function`?null:V_(a)}},d=(e,n,r)=>{if(z_(e)){let t=e.shown();return{id:e.id,label:e.label,group:n,kind:`part`,sits:null,on:t,visible:t,opacity:null,legend:null,swatch:e.swatch??null,movable:!1,note:e.note,...e.radio?{radio:e.radio}:{}}}let i=e,a=i.forced!==!1,{legend:o,swatch:s}=u(i);return{id:i.id,label:i.props.label??i.id,group:n,kind:i.kind,sits:i.sits,on:a,visible:a&&i.target>0,opacity:i.fades?i.props.opacity??1:null,legend:t.legend===!1?null:o,swatch:s,movable:r}},f=()=>({title:n,groups:l().map(e=>{let n=t.reorder!==!1&&e.entries.filter(e=>!z_(e)).length>1;return{name:e.name,items:e.entries.map(t=>d(t,e.name,n&&!z_(t)&&t.sits!==null))}})}),p=(t,n)=>{for(let i of l())for(let a of i.entries)a.id===t&&(z_(a)?a.show(n):n?e.release(t,{seconds:r}):e.hide(t,{seconds:r}));N()},m=(t,n)=>e.setOpacity(t,n),h=(t,n)=>{let r=l().find(e=>e.name===t);if(!r)return;let i=r.entries.filter(e=>!z_(e)&&e.sits!==null).map(e=>e.id),a=n.filter(e=>i.includes(e));for(let e of i)a.includes(e)||a.push(e);e.reorder([...a].reverse())},g=null,_=t.folded??(typeof innerWidth==`number`&&innerWidth<600),v=!1,y=/* @__PURE__ */ new Set,b=0,x=e.renderer?.canvas,S=t.into??(x&&`parentElement`in x?x.parentElement:null),C=e=>{if(e.legend?.kind===`image`)return`<span class="al-sw al-sw-pic"></span>`;let t=e.swatch??(e.legend?.kind===`continuous`?null:e.legend?.items[Math.floor((e.legend.items.length-1)/2)]?.color)??null;if(e.legend?.kind===`continuous`&&e.legend.stops?.length)return`<span class="al-sw al-sw-ramp" style="background:linear-gradient(90deg,${e.legend.stops.join(`,`)})"></span>`;if(!t)return`<span class="al-sw al-sw-none"></span>`;let n=e.kind===`line`?`line`:e.kind===`point`?`dot`:e.kind===`label`?`text`:`fill`;return n===`text`?`<span class="al-sw al-sw-text" style="color:${t}">Aa</span>`:`<span class="al-sw al-sw-${n}" style="--c:${t}"></span>`},w=e=>e.kind===`image`?`<img class="al-legpic" src="${B_(e.image??``)}" alt="legend" loading="lazy">`:e.kind===`continuous`?`<div class="al-ramp" style="background:linear-gradient(90deg,${(e.stops??[]).join(`,`)})"></div><div class="al-ends">${e.items.map(e=>`<span>${B_(e.label)}</span>`).join(``)}</div>`:`${e.items.map(e=>`<div class="al-class"><span class="al-sw al-sw-fill" style="--c:${e.color??`#888`}"></span><span>${B_(e.label)}</span></div>`).join(``)}${e.missing?`<div class="al-class al-dim"><span class="al-sw al-sw-fill" style="--c:${e.missing}"></span><span>no data</span></div>`:``}`,T=e=>{let n=t.opacity!==!1&&e.opacity!==null||t.legend!==!1&&e.legend,r=y.has(e.id);return`<li class="al-row${e.on?``:` al-off`}${e.on&&!e.visible?` al-out`:``}${r?` al-open`:``}" data-id="${B_(e.id)}">
      <div class="al-line">${e.movable?`<span class="al-grip" title="drag to move it up or down" aria-label="move">⋮⋮</span>`:`<span class="al-grip al-nogrip"></span>`}
        <label class="al-name"><input type="${e.radio?`radio`:`checkbox`}"${e.radio?` name="al-${B_(e.radio)}"`:``} ${e.on?`checked`:``} aria-label="show ${B_(e.label)}">${C(e)}<span class="al-text">${B_(e.label)}${e.note?`<small>${B_(e.note)}</small>`:``}<small class="al-why">not at this zoom</small></span></label>
        ${t.modes&&e.sits&&e.sits!==`free`?`<button class="al-mode" data-mode title="where it sits with the twin: tap to change">${e.sits}</button>`:``}
        ${n?`<button class="al-more" data-more aria-expanded="${r}" title="opacity and legend">${r?`▴`:`▾`}</button>`:``}
      </div>
      ${n&&r?`<div class="al-detail">${t.opacity!==!1&&e.opacity!==null?`<label class="al-op"><span>opacity</span><input type="range" min="0" max="1" step="0.05" value="${e.opacity}" data-op><output>${Math.round(e.opacity*100)}%</output></label>`:``}${t.legend!==!1&&e.legend?`<div class="al-legend">${w(e.legend)}</div>`:``}</div>`:``}
    </li>`},E=()=>{if(!g||v)return;let e=f(),t=e.groups.reduce((e,t)=>e+t.items.filter(e=>e.on).length,0);g.classList.toggle(`al-folded`,_),g.innerHTML=`<button class="al-head" aria-expanded="${!_}"><span class="al-icon">☰</span><span>${B_(e.title)}</span><span class="al-count">${t}</span><span class="al-chev">${_?`▾`:`▴`}</span></button>
      <div class="al-body">${e.groups.map(e=>`<section class="al-group" data-g="${B_(e.name)}">${e.name?`<header><label><input type="checkbox" data-all ${e.items.some(e=>e.on)?`checked`:``} aria-label="show all of ${B_(e.name)}"><span>${B_(e.name)}</span></label></header>`:``}<ol>${e.items.map(T).join(``)}</ol></section>`).join(``)}${o?O():``}</div>`},D={open:!1,url:o?.services?.[0]??``,kind:`auto`,busy:!1,error:``,picks:[],filter:``},O=()=>{if(!D.open)return`<button class="al-addbtn" data-add>＋ Add a layer from a map or feature service</button>`;let e=D.filter.toLowerCase(),t=D.picks.filter(t=>!e||t.title.toLowerCase().includes(e)||t.name.toLowerCase().includes(e)).slice(0,60);return`<div class="al-add"><div class="al-addrow"><input class="al-url" data-url type="url" inputmode="url" placeholder="https://…/wms or …/wfs" value="${B_(D.url)}" list="al-services">${o?.services?.length?`<datalist id="al-services">${o.services.map(e=>`<option value="${B_(e)}">`).join(``)}</datalist>`:``}</div>
      <div class="al-addrow"><select data-kind aria-label="service kind">${[`auto`,`wms`,`wfs`].map(e=>`<option value="${e}" ${D.kind===e?`selected`:``}>${e===`auto`?`WMS or WFS`:e.toUpperCase()}</option>`).join(``)}</select><button class="al-go" data-list ${D.busy?`disabled`:``}>${D.busy?`Reading…`:`List its layers`}</button><button class="al-x" data-close aria-label="close">✕</button></div>
      ${D.error?`<div class="al-err">${B_(D.error)}</div>`:``}
      ${D.picks.length?`<input class="al-filter" data-filter placeholder="filter ${D.picks.length} layers" value="${B_(D.filter)}"><ul class="al-picks">${t.map(e=>`<li><button data-pick="${B_(`${e.service}|${e.name}`)}"><b>${e.service.toUpperCase()}</b> ${B_(e.title)}<small>${B_(e.name)}${e.geometry?` · ${e.geometry}`:``}</small></button></li>`).join(``)}</ul>`:``}</div>`},k=()=>({headers:o?.headers,fetch:o?.fetch,proxy:o?.proxy}),A=async(e,t=`auto`)=>{let n=[],r=[],i=t===`wms`||t===`auto`&&!/wfs/i.test(e),a=t===`wfs`||t===`auto`&&!/wms/i.test(e);if(i&&await Rn(e,k()).then(t=>{for(let r of t.layers)n.push({service:`wms`,url:e,name:r.name,title:r.title,geometry:null})}).catch(e=>r.push(`WMS: ${e.message}`)),a&&await jn(e,k()).then(t=>{for(let r of t.layers)n.push({service:`wfs`,url:e,name:r.name,title:r.title,geometry:null})}).catch(e=>r.push(`WFS: ${e.message}`)),!n.length)throw Error(r.join(` · `)||`no layers`);return n},j=0,M=async t=>{let n,r=`${t.service}-${t.name.replace(/[^A-Za-z0-9_-]+/g,`_`)}`;if(o?.make)n=await o.make(t);else if(t.service===`wms`)n=tl({id:r,label:t.title,source:tl.wms(t.url,{layers:t.name,single:!0,...k()}),opacity:.7});else{let i=t.geometry??W_((await Hn(t.url,t.name,k()).catch(()=>null))?.geometry?.type),a=U_[j++%U_.length],o=e.state.view,s=111320*Math.max(.05,Math.cos(o.lat*Math.PI/180)),c=.75*Math.max(o.width,o.height)*o.pixelSize*(1+Math.min(2,o.tilt/30)),l=[o.lon-c/s,Math.max(-85,o.lat-c/110574),o.lon+c/s,Math.min(85,o.lat+c/110574)],u=Kt(t.url,{typeName:t.name,orExtent:l,most:5e3,...k()}),d=r;n=i===`point`?Yo({id:d,label:t.title,data:u,color:a,size:7,outline:{color:`#ffffff`,width:1.5}}):i===`line`?Ro({id:d,label:t.title,data:u,color:a,width:2}):Uo({id:d,label:t.title,data:u,color:`${a}66`,outline:{color:a,width:1.5}})}return e.list.some(e=>e.id===n.id)&&e.remove(n.id),e.add(n),c.push(n.id),N(),n},N=()=>{b||=setTimeout(()=>{b=0,E()},0)},P=null;if(S&&typeof document<`u`){if(K_(),g=document.createElement(`div`),g.className=`aslnut-layers${t.className?` ${t.className}`:``}`,!t.into){let[e,n]=(t.position??`top-right`).split(`-`);g.classList.add(`al-float`,`al-${e}`),g.style[e]=`calc(${e===`bottom`?26:8}px + env(safe-area-inset-${e}, 0px))`,g.style[n]=`8px`,getComputedStyle(S).position===`static`&&(S.style.position=`relative`)}S.appendChild(g),E(),P=e.on(`elements`,e=>{e.what===`opacity`&&g?.contains(document.activeElement)||N()}),g.addEventListener(`click`,e=>{let t=e.target;if(t.closest(`.al-head`)){_=!_,E();return}if(t.closest(`[data-add]`)){D.open=!0,E(),g?.querySelector(`[data-url]`)?.focus();return}if(t.closest(`[data-close]`)){D={...D,open:!1,error:``,picks:[]},E();return}if(t.closest(`[data-list]`)){D.url=g?.querySelector(`[data-url]`)?.value.trim()??D.url,D.busy=!0,D.error=``,D.picks=[],E(),A(D.url,D.kind).then(e=>{D.picks=e}).catch(e=>{D.error=e.message.slice(0,300)}).finally(()=>{D.busy=!1,E()});return}let n=t.closest(`[data-pick]`)?.dataset.pick;if(n){let[e,...t]=n.split(`|`),r=D.picks.find(n=>n.service===e&&n.name===t.join(`|`));r&&(D={...D,open:!1,picks:[]},M(r).catch(e=>{D={...D,open:!0,error:e.message},E()}));return}let r=t.closest(`.al-row`)?.dataset.id;if(t.closest(`[data-more]`)&&r)y.has(r)?y.delete(r):y.add(r),E();else if(t.closest(`[data-mode]`)&&r){let e=i(r);if(!e)return;let t=e.sits,n=H_[(H_.indexOf(t)+1)%H_.length];e.setProps({drape:n}),E()}}),g.addEventListener(`change`,e=>{let t=e.target;if(t.dataset.kind!==void 0){D.kind=t.value;return}if(t.dataset.all!==void 0){let e=l().find(e=>e.name===t.closest(`.al-group`)?.dataset.g);for(let n of e?.entries??[])z_(n)&&n.radio||p(n.id,t.checked);return}let n=t.closest(`.al-row`)?.dataset.id;n&&t.type===`checkbox`&&p(n,t.checked),n&&t.type===`radio`&&t.checked&&p(n,!0)}),g.addEventListener(`input`,e=>{let t=e.target,n=t.closest(`.al-row`)?.dataset.id;if(t.dataset.url!==void 0)D.url=t.value;else if(t.dataset.filter!==void 0){D.filter=t.value;let e=t.selectionStart;E();let n=g?.querySelector(`[data-filter]`);n&&(n.focus(),n.setSelectionRange(e,e))}else if(n&&t.dataset.op!==void 0){m(n,Number(t.value));let e=t.parentElement?.querySelector(`output`);e&&(e.textContent=`${Math.round(Number(t.value)*100)}%`)}}),g.addEventListener(`pointerdown`,e=>{let t=e.target.closest(`.al-grip:not(.al-nogrip)`);if(!t)return;let n=t.closest(`.al-row`),r=n.parentElement,i=n.closest(`.al-group`).dataset.g??``;e.preventDefault(),t.setPointerCapture(e.pointerId),v=!0;let a=[...r.children],o=a.indexOf(n),s=n.getBoundingClientRect(),c=e.clientY,l=a.map(e=>{let t=e.getBoundingClientRect();return t.top+t.height/2}),u=o;n.classList.add(`al-drag`);let d=e=>{let t=e.clientY-c;n.style.transform=`translateY(${t}px)`;let r=s.top+s.height/2+t;u=0;for(let e=0;e<l.length;e++)e!==o&&l[e]<r&&u++;a.forEach((e,t)=>{if(e===n)return;let r=o<u?t>o&&t<=u?-s.height:0:t>=u&&t<o?s.height:0;e.style.transform=r?`translateY(${r}px)`:``})},f=()=>{if(t.removeEventListener(`pointermove`,d),t.removeEventListener(`pointerup`,f),t.removeEventListener(`pointercancel`,f),a.forEach(e=>{e.style.transform=``}),n.classList.remove(`al-drag`),v=!1,u!==o){let e=a.map(e=>e.dataset.id);e.splice(u,0,e.splice(o,1)[0]),h(i,e)}E()};t.addEventListener(`pointermove`,d),t.addEventListener(`pointerup`,f),t.addEventListener(`pointercancel`,f)})}return{el:g,describe:f,update:E,show:p,setOpacity:m,restack:h,listService:A,addFrom:M,fold:e=>{_=e??!_,E()},remove:()=>{P?.(),clearTimeout(b),g?.remove(),g=null}}}function K_(){if(document.getElementById(`aslnut-layers-style`))return;let e=document.createElement(`style`);e.id=`aslnut-layers-style`,e.textContent=`
.aslnut-layers { --al-bg: rgba(12,16,24,.88); --al-line: rgba(255,255,255,.12); --al-fg: #e6ebf0; --al-dim: #93a1af; --al-accent: #38bdf8;
  font: 13px/1.35 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; color: var(--al-fg); width: min(300px, calc(100vw - 16px)); z-index: 12; user-select: none; }
.aslnut-layers.al-float { position: absolute; }
.aslnut-layers .al-head { all: unset; box-sizing: border-box; display: flex; align-items: center; gap: 8px; width: 100%; min-height: 40px; padding: 8px 12px; background: var(--al-bg); border: 1px solid var(--al-line); border-radius: 12px; cursor: pointer; font-weight: 600; backdrop-filter: blur(8px); }
.aslnut-layers.al-folded { width: auto; }
.aslnut-layers.al-folded .al-head { width: auto; }
.aslnut-layers:not(.al-folded) .al-head { border-radius: 12px 12px 0 0; }
.aslnut-layers.al-bottom { display: flex; flex-direction: column-reverse; align-items: flex-end; }
.aslnut-layers.al-bottom:not(.al-folded) .al-head { border-radius: 0 0 12px 12px; }
.aslnut-layers.al-bottom .al-body { border-top: 1px solid var(--al-line); border-bottom: 0; border-radius: 12px 12px 0 0; width: 100%; box-sizing: border-box; }
.aslnut-layers .al-count { margin-left: auto; min-width: 20px; padding: 1px 6px; border-radius: 10px; background: rgba(56,189,248,.18); color: var(--al-accent); font-size: 11px; text-align: center; }
.aslnut-layers .al-chev { color: var(--al-dim); font-size: 11px; }
.aslnut-layers.al-folded .al-body { display: none; }
.aslnut-layers .al-body { background: var(--al-bg); border: 1px solid var(--al-line); border-top: 0; border-radius: 0 0 12px 12px; max-height: min(64vh, calc(100vh - 120px)); overflow: auto; overscroll-behavior: contain; backdrop-filter: blur(8px); padding: 4px 0 6px; }
.aslnut-layers .al-group header { padding: 8px 12px 4px; color: var(--al-dim); font-size: 11px; text-transform: uppercase; letter-spacing: .06em; }
.aslnut-layers .al-group header label { display: flex; align-items: center; gap: 8px; cursor: pointer; }
.aslnut-layers ol { list-style: none; margin: 0; padding: 0; }
.aslnut-layers .al-row { position: relative; border-top: 1px solid rgba(255,255,255,.04); background: transparent; transition: transform .12s ease; }
.aslnut-layers .al-row.al-drag { z-index: 2; background: rgba(56,189,248,.14); transition: none; box-shadow: 0 4px 14px rgba(0,0,0,.4); }
.aslnut-layers .al-line { display: flex; align-items: center; gap: 4px; min-height: 44px; padding: 0 6px 0 2px; }
.aslnut-layers .al-grip { width: 24px; flex: none; text-align: center; color: var(--al-dim); cursor: grab; touch-action: none; font-size: 12px; letter-spacing: -2px; padding: 12px 0; }
.aslnut-layers .al-nogrip { cursor: default; }
.aslnut-layers .al-name { flex: 1; min-width: 0; display: flex; align-items: center; gap: 8px; cursor: pointer; padding: 6px 0; }
.aslnut-layers input[type=checkbox], .aslnut-layers input[type=radio] { width: 18px; height: 18px; margin: 0; accent-color: var(--al-accent); flex: none; }
.aslnut-layers .al-text { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.aslnut-layers .al-text small { display: block; color: var(--al-dim); font-size: 11px; }
.aslnut-layers .al-why { display: none !important; }
.aslnut-layers .al-out .al-why { display: block !important; }
.aslnut-layers .al-off .al-text, .aslnut-layers .al-out .al-text { color: var(--al-dim); }
.aslnut-layers .al-sw { width: 16px; height: 16px; flex: none; border-radius: 4px; display: inline-block; }
.aslnut-layers .al-sw-fill { background: var(--c); }
.aslnut-layers .al-sw-line { height: 4px; border-radius: 2px; background: var(--c); }
.aslnut-layers .al-sw-dot { width: 12px; height: 12px; margin: 2px; border-radius: 50%; background: var(--c); box-shadow: 0 0 0 2px rgba(0,0,0,.5); }
.aslnut-layers .al-sw-text { font-weight: 700; font-size: 12px; text-align: center; line-height: 16px; }
.aslnut-layers .al-sw-ramp { height: 10px; }
.aslnut-layers .al-sw-none { background: transparent; }
.aslnut-layers .al-mode, .aslnut-layers .al-more { all: unset; box-sizing: border-box; min-width: 32px; height: 32px; padding: 0 8px; border-radius: 8px; text-align: center; line-height: 32px; color: var(--al-dim); cursor: pointer; font-size: 12px; flex: none; }
.aslnut-layers .al-mode { border: 1px solid var(--al-line); height: 26px; line-height: 24px; font-size: 11px; }
.aslnut-layers .al-mode:hover, .aslnut-layers .al-more:hover { background: rgba(255,255,255,.08); color: var(--al-fg); }
.aslnut-layers .al-detail { padding: 2px 12px 10px 34px; display: grid; gap: 8px; }
.aslnut-layers .al-op { display: grid; grid-template-columns: auto 1fr 38px; align-items: center; gap: 8px; color: var(--al-dim); font-size: 12px; }
.aslnut-layers .al-op input { width: 100%; accent-color: var(--al-accent); height: 24px; }
.aslnut-layers .al-op output { text-align: right; font-variant-numeric: tabular-nums; }
.aslnut-layers .al-legend { display: grid; gap: 3px; font-size: 12px; }
.aslnut-layers .al-class { display: flex; align-items: center; gap: 8px; }
.aslnut-layers .al-class .al-sw { width: 12px; height: 12px; border-radius: 3px; }
.aslnut-layers .al-dim { color: var(--al-dim); }
.aslnut-layers .al-ramp { height: 10px; border-radius: 3px; }
.aslnut-layers .al-ends { display: flex; justify-content: space-between; color: var(--al-dim); font-size: 11px; }
.aslnut-layers .al-sw-pic { background: linear-gradient(135deg, #e4572e 0 33%, #ffc914 33% 66%, #17bebb 66%); opacity: .8; }
.aslnut-layers .al-legpic { max-width: 100%; max-height: 220px; background: #fff; border-radius: 4px; padding: 4px; justify-self: start; }
.aslnut-layers .al-addbtn { all: unset; box-sizing: border-box; display: block; width: calc(100% - 16px); margin: 8px; padding: 10px 12px; border: 1px dashed var(--al-line); border-radius: 10px; color: var(--al-accent); cursor: pointer; text-align: center; font-size: 12px; }
.aslnut-layers .al-add { margin: 8px; padding: 8px; border: 1px solid var(--al-line); border-radius: 10px; display: grid; gap: 6px; user-select: text; }
.aslnut-layers .al-addrow { display: flex; gap: 6px; }
.aslnut-layers .al-add input, .aslnut-layers .al-add select { box-sizing: border-box; min-width: 0; flex: 1; height: 34px; padding: 0 8px; border-radius: 8px; border: 1px solid var(--al-line); background: rgba(255,255,255,.06); color: var(--al-fg); font: inherit; font-size: 16px; }
.aslnut-layers .al-add select { flex: none; width: auto; font-size: 13px; }
.aslnut-layers .al-go, .aslnut-layers .al-x { all: unset; box-sizing: border-box; height: 34px; padding: 0 10px; border-radius: 8px; background: rgba(56,189,248,.18); color: var(--al-accent); cursor: pointer; font-size: 12px; white-space: nowrap; }
.aslnut-layers .al-x { background: transparent; color: var(--al-dim); }
.aslnut-layers .al-err { color: #fca5a5; font-size: 12px; word-break: break-word; }
.aslnut-layers .al-picks { list-style: none; margin: 0; padding: 0; max-height: 40vh; overflow: auto; }
.aslnut-layers .al-picks button { all: unset; box-sizing: border-box; display: block; width: 100%; padding: 8px 6px; border-top: 1px solid rgba(255,255,255,.05); cursor: pointer; font-size: 12px; }
.aslnut-layers .al-picks button:hover { background: rgba(255,255,255,.06); }
.aslnut-layers .al-picks b { color: var(--al-accent); font-size: 10px; margin-right: 4px; }
.aslnut-layers .al-picks small { display: block; color: var(--al-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
`,document.head.appendChild(e)}function q_(e,t){let n=t?ss.length:1,r=[`items`,`times`,`values`,`texts`,`glyphs`,`icons`,`fills`,`state`,`claims`];return`${t?ys(t):nn}
struct U { place: vec4f, size: vec4f, lay: vec4f, clock: vec4f, look: vec4f, background: vec4f, room: vec4f, more: vec4f, bar: vec4f, dark: vec4f, light: vec4f, badge: vec4f, fill: vec4f, badgeOn: vec4f, glyph: vec4f, claim: vec4f, hold: vec4f, header: vec4f, channels: vec4f, rows: array<vec4f, 6>, rowsBar: array<vec4f, 6>, labels: array<vec4f, 6>, labelSize: array<vec4f, 6> }
@group(0) @binding(${t?2:1}) var<uniform> u: U;
${t?``:`@group(1) @binding(0) var places: texture_2d<f32>;`}
${r.map((e,t)=>`@group(1) @binding(${n+t}) var ${e}: texture_2d<f32>;`).join(`
`)}
@group(1) @binding(${n+r.length}) var smp: sampler;
${t?os:as}
fn cardItem(e: i32, k: i32) -> vec4f {
  let i = e * i32(u.more.x) + k;
  return textureLoad(items, vec2i(i % 4096, i / 4096), 0);
}
// its width in the layout's units (CSS px at 120 wide): its own when it fits what it holds, else the set's
fn cardWide(e: i32) -> f32 {
  let w = cardItem(e, 2).w;
  return select(120.0, w, w > 0.5);
}
// a decision as kept (a texel a card, rgba8): shown (128) and its fade (0..127); when it last changed (ms, two
// bytes); since when its room is covered (10 ms steps + 1; 0: it is not; 255: not decided yet)
fn stateFade(s: vec4f) -> f32 {
  let r = round(s.x * 255.0);
  return (r % 128.0) / 127.0;
}
fn stateShown(s: vec4f) -> bool {
  return round(s.x * 255.0) >= 128.0;
}
// not shown now (or not decided yet): after its set's cards shown in the room's order
fn cardLater(e: i32) -> bool {
  let s = textureLoad(state, vec2i(e % 4096, e / 4096), 0);
  return !(stateShown(s) && s.w < 0.99);
}
// its place in the room's one order: the top set first; in a set its cards shown first (a winner keeps its room),
// then its priority, then its id
fn cardOrder(e: i32) -> f32 {
  return u.glyph.w + cardItem(e, 0).w + select(0.0, 262144.0, cardLater(e));
}
fn cardTime(sl: i32) -> f32 {
  return textureLoad(times, vec2i(sl % 4096, sl / 4096), 0).x;
}
// the item's sample at the clock: (its slot, the next, how far on), a binary search over its slots
fn cardSample(e: i32) -> vec3f {
  let m = cardItem(e, 0);
  let a = i32(m.x);
  let n = i32(m.y);
  let t = u.clock.x;
  if (n <= 1 || t <= cardTime(a)) {
    return vec3f(f32(a), f32(a), 0.0);
  }
  if (t >= cardTime(a + n - 1)) {
    return vec3f(f32(a + n - 1), f32(a + n - 1), 0.0);
  }
  var lo = 0;
  var hi = n - 1;
  for (var k = 0; k < 24; k++) {
    if (hi - lo <= 1) {
      break;
    }
    let mid = (lo + hi) / 2;
    if (cardTime(a + mid) <= t) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  let ta = cardTime(a + lo);
  let tb = cardTime(a + hi);
  return vec3f(f32(a + lo), f32(a + hi), select(0.0, clamp((t - ta) / (tb - ta), 0.0, 1.0), tb > ta));
}
fn cardValueAt(sl: i32, c: i32) -> f32 {
  let Q = i32(u.channels.x);
  let i = sl * Q + c / 4;
  let v = textureLoad(values, vec2i(i % 4096, i / 4096), 0);
  let k = c % 4;
  return select(select(select(v.w, v.z, k == 2), v.y, k == 1), v.x, k == 0);
}
// a channel's value at the clock (c < 0: none, 0)
fn cardChannel(sm: vec3f, c: f32) -> f32 {
  if (c < -0.5) {
    return 0.0;
  }
  let ci = i32(c + 0.5);
  return mix(cardValueAt(i32(sm.x), ci), cardValueAt(i32(sm.y), ci), sm.z);
}
// its height (CSS px at 120 wide) at the governor's step: whole, its header only, or none (a dot)
fn cardHeight() -> f32 {
  let st = u.clock.y;
  let head = select(0.0, u.lay.y, u.header.x + u.header.y > 0.5);
  let body = select(0.0, u.lay.z * u.lay.w, st < 0.5);
  return 2.0 * u.lay.x + head + body;
}
// a card's room on the screen (device px from the top left: x0, y0, x1, y1; its pointer's tip at (x, y) =
// ((x0 + x1) / 2, y1 + the pointer)) and its anchor's depth; w: 0 when it is not placed or off the front
struct Room { box: vec4f, depth: f32, ok: f32, k: f32 }
fn cardRoom(e: i32) -> Room {
  var r: Room;
  r.box = vec4f(0.0);
  r.depth = 0.0;
  r.ok = 0.0;
  r.k = 1.0;
  let a = anchorOf(e);
  if (a.w < 0.5 || anchorBehind(a.xyz)) {
    return r;
  }
  let clip = frame.viewProj * vec4f(a.xyz, 1.0);
  if (clip.w < 0.01) {
    return r;
  }
  let ndc = clip.xyz / clip.w;
  let px = vec2f(round((ndc.x * 0.5 + 0.5) * frame.size.x), round((0.5 - ndc.y * 0.5) * frame.size.y));
  let w = sizePx(u.size.w, vec4f(u.size.xyz, 0.0), a.xyz);
  let k = w / 120.0;
  let dpr = frame.size.z;
  let W = round(cardWide(e) * k * dpr);
  let H = round(cardHeight() * k * dpr);
  let y1 = px.y - round((u.look.y + 2.0) * k * dpr);
  let x0 = px.x - round(W * 0.5);
  r.box = vec4f(x0, y1 - H, x0 + W, y1);
  r.depth = clamp(ndc.z, 0.0, 1.0);
  r.ok = 1.0;
  r.k = k;
  return r;
}
// device px from the top left → clip (the picking pass's region zoomed onto: clipOut)
fn cardClip(p: vec2f, z: f32) -> vec4f {
  return clipOut(vec4f(p.x / frame.size.x * 2.0 - 1.0, 1.0 - p.y / frame.size.y * 2.0, z, 1.0));
}
${e===`cards`?`
struct O { @builtin(position) pos: vec4f, @location(0) local: vec2f, @location(1) @interpolate(flat) dims: vec4f, @location(2) @interpolate(flat) ra: vec4f, @location(3) @interpolate(flat) rb: vec4f, @location(4) @interpolate(flat) rc: vec4f, @location(5) @interpolate(flat) fillc: vec4f, @location(6) @interpolate(flat) misc: vec4f, @location(7) @interpolate(flat) head: vec4f, @location(8) @interpolate(flat) headSize: vec4f, @location(9) @interpolate(flat) id: f32 }
fn cardCorner(v: i32) -> vec2f {
  let x = select(0.0, 1.0, v == 1 || v == 3);
  let y = select(0.0, 1.0, v >= 2);
  return vec2f(x, y);
}
fn cardRamp(v: f32) -> vec4f {
  let t = clamp((v - u.fill.y) / max(u.fill.z - u.fill.y, 1e-9), 0.0, 1.0);
  return textureLoad(fills, vec2i(i32(t * 255.0 + 0.5), 0), 0);
}
// the collision's state of a card: shown 0..1 (its fade; 1 without collision)
fn cardShown(e: i32) -> f32 {
  if (u.claim.w < 0.5) {
    return 1.0;
  }
  return stateFade(textureLoad(state, vec2i(e % 4096, e / 4096), 0));
}
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let e = i32(iid);
  let isDot = vid >= 4u;
  let corner = cardCorner(i32(vid % 4u));
  var o: O;
  o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
  o.local = vec2f(0.0);
  o.dims = vec4f(0.0);
  o.ra = vec4f(0.0);
  o.rb = vec4f(0.0);
  o.rc = vec4f(0.0);
  o.fillc = vec4f(0.0);
  o.misc = vec4f(0.0);
  o.head = vec4f(0.0);
  o.headSize = vec4f(0.0);
  o.id = f32(e);
  let r = cardRoom(e);
  if (r.ok < 0.5) {
    return o;
  }
  let seen = cardShown(e);
  let sm = cardSample(e);
  let fv = cardChannel(sm, u.fill.x);
  let fc = select(u.background, cardRamp(fv), u.fill.w > 0.5);
  let dpr = frame.size.z;
  let tip = vec2f((r.box.x + r.box.z) * 0.5, r.box.w + round((u.look.y + 2.0) * r.k * dpr));
  if (isDot) {
    // a dot at its place: a card that lost its room, or one the governor stepped down to it
    let lost = select(1.0 - cardShown(e), 1.0, u.clock.y > 1.5);
    if (lost * u.look.z <= 0.0) {
      return o;
    }
    let d = round(u.look.z * dpr);
    let p = tip + (corner - vec2f(0.5)) * d;
    o.pos = cardClip(p, r.depth);
    o.local = (corner - vec2f(0.5)) * 2.0;
    o.dims = vec4f(-1.0, d, 0.0, lost * u.look.x);
    o.fillc = select(u.dark, fc, u.fill.w > 0.5);
    return o;
  }
  if (u.clock.y > 1.5 || seen <= 0.0) {
    return o;
  }
  // (its quad: the card, its pointer under it, its badge over its top right corner)
  let m = round(8.0 * r.k * dpr);
  let box = vec4f(r.box.x, r.box.y - m, r.box.z + m, tip.y);
  let p = mix(box.xy, box.zw, corner);
  o.pos = cardClip(p, r.depth);
  o.local = (p - r.box.xy) / dpr / r.k;
  o.dims = vec4f((r.box.z - r.box.x) / dpr / r.k, (r.box.w - r.box.y) / dpr / r.k, r.k * dpr, seen * u.look.x);
  o.ra = vec4f(cardChannel(sm, u.rows[0].x), cardChannel(sm, u.rows[0].y), cardChannel(sm, u.rows[1].x), cardChannel(sm, u.rows[1].y));
  o.rb = vec4f(cardChannel(sm, u.rows[2].x), cardChannel(sm, u.rows[2].y), cardChannel(sm, u.rows[3].x), cardChannel(sm, u.rows[3].y));
  o.rc = vec4f(cardChannel(sm, u.rows[4].x), cardChannel(sm, u.rows[4].y), cardChannel(sm, u.rows[5].x), cardChannel(sm, u.rows[5].y));
  o.fillc = fc;
  let it = cardItem(e, 0);
  o.misc = vec4f(it.z, cardChannel(sm, u.badgeOn.x), select(0.0, 1.0, cardChannel(sm, u.badgeOn.y) > 0.5 && u.badgeOn.z > 0.5), 0.0);
  o.head = cardItem(e, 1);
  o.headSize = cardItem(e, 2);
  return o;
}
// a glyph's coverage at (x, y) in its cell (glyph heights), its code (0–9, 10 '.', 11 '−', 12 '/', 13 '%', 14 '+')
fn glyphCover(code: i32, x: f32, y: f32) -> f32 {
  if (code < 0 || x < 0.0 || x > u.glyph.y || y < 0.0 || y > 1.0) {
    return 0.0;
  }
  let uv = vec2f((f32(code) * u.glyph.y + x) / (u.glyph.z * u.glyph.y), y);
  return textureSampleLevel(glyphs, smp, uv, 0.0).a;
}
// the digits of n (n ≥ 0): how many (one at least)
fn digitCount(n: i32) -> i32 {
  var c = 1;
  var m = n / 10;
  for (var k = 0; k < 9; k++) {
    if (m <= 0) {
      break;
    }
    c++;
    m = m / 10;
  }
  return c;
}
fn powTen(d: i32) -> i32 {
  return select(select(1, 10, d == 1), 100, d >= 2);
}
// the r-th character from the right of v written with dec decimals (−1: none)
fn numberChar(v: f32, dec: i32, r: i32) -> i32 {
  let p = powTen(dec);
  let iv = i32(round(min(abs(v), 9999999.0) * f32(p)));
  let ip = iv / p;
  let fp = iv - ip * p;
  if (r < dec) {
    return (fp / powTen(r)) % 10;
  }
  var k = r - dec;
  if (dec > 0) {
    if (k == 0) {
      return 10;
    }
    k = k - 1;
  }
  let n = digitCount(ip);
  if (k < n) {
    var q = ip;
    for (var j = 0; j < 9; j++) {
      if (j >= k) {
        break;
      }
      q = q / 10;
    }
    return q % 10;
  }
  if (k == n && v < 0.0 && iv > 0) {
    return 11;
  }
  return -1;
}
// how many characters v takes with dec decimals
fn numberLength(v: f32, dec: i32) -> i32 {
  let p = powTen(dec);
  let iv = i32(round(min(abs(v), 9999999.0) * f32(p)));
  return digitCount(iv / p) + select(0, dec + 1, dec > 0) + select(0, 1, v < 0.0 && iv > 0);
}
// a value (and its 'of': v/of) right-aligned at x1, its glyphs h px tall: the coverage at q (CSS px at 120 wide)
fn numberCover(q: vec2f, x1: f32, y0: f32, h: f32, v: f32, ofv: f32, hasOf: bool, dec: i32) -> f32 {
  let adv = u.glyph.x * h;
  let r = i32(floor((x1 - q.x) / adv));
  if (r < 0 || q.y < y0 || q.y > y0 + h) {
    return 0.0;
  }
  var code = -1;
  var rr = r;
  if (hasOf) {
    let no = numberLength(ofv, 0);
    if (rr < no) {
      code = numberChar(ofv, 0, rr);
    } else if (rr == no) {
      code = 12;
    }
    rr = rr - no - 1;
  }
  if (code < 0 && rr >= 0) {
    code = numberChar(v, dec, rr);
  }
  let x = (x1 - f32(r + 1) * adv - (u.glyph.y * h - adv) * 0.5);
  return glyphCover(code, (q.x - x) / h, (q.y - y0) / h);
}
// a picture of the texts' atlas: its uv rect, drawn into the box at (x0, y0) of w × h CSS px (at 120 wide)
fn pictureCover(q: vec2f, rect: vec4f, x0: f32, y0: f32, w: f32, h: f32) -> f32 {
  let t = (q - vec2f(x0, y0)) / vec2f(w, h);
  if (t.x < 0.0 || t.y < 0.0 || t.x > 1.0 || t.y > 1.0) {
    return 0.0;
  }
  return textureSampleLevel(texts, smp, mix(rect.xy, rect.zw, t), 0.0).a;
}
fn rowValue(i: O, k: i32) -> vec2f {
  if (k < 2) {
    return select(i.ra.zw, i.ra.xy, k == 0);
  }
  if (k < 4) {
    return select(i.rb.zw, i.rb.xy, k == 2);
  }
  return select(i.rc.zw, i.rc.xy, k == 4);
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let q = i.local;
  let W = i.dims.x;
  // a dot: a disc with a light rim
  if (i.dims.x < -0.5) {
    let r = length(q);
    let px = 2.0 / max(i.dims.y, 1.0);
    let disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
    let rim = smoothstep(1.0 - px * 4.0, 1.0 - px * 3.0, r);
    let a = disc * i.dims.w;
    if (a <= 0.004) {
      discard;
    }
    if (frame.pick.x > 0.5) {
      return pickOut(u.clock.z, i.id);
    }
    return vec4f(themed(mix(i.fillc.rgb, u.dark.rgb, rim * 0.85), 1.0), a);
  }
  let pad = u.lay.x;
  let H = cardHeight();
  let rad = u.look.w;
  // the panel: a rounded box, its pointer under its middle
  let c = clamp(q, vec2f(rad), vec2f(W - rad, H - rad));
  let inBox = length(q - c) <= rad;
  let pt = u.look.y;
  let inTip = q.y >= H - 0.5 && q.y <= H + pt && abs(q.x - W * 0.5) <= (H + pt - q.y) * 0.9;
  // the badge: a disc over its top right corner
  let bc = vec2f(W - 3.0, 3.0);
  let br = 8.0;
  let inBadge = i.misc.z > 0.5 && length(q - bc) <= br;
  if (!inBox && !inTip && !inBadge) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    return pickOut(u.clock.z, i.id);
  }
  let bg = i.fillc;
  let lum = dot(bg.rgb, vec3f(0.299, 0.587, 0.114));
  let ink = select(u.light, u.dark, lum > 0.55);
  var rgb = bg.rgb;
  var alpha = bg.a;
  // its edge: a hairline a shade darker
  let edge = length(q - c) - rad;
  if (inBox && (edge > -1.0 / i.dims.z * 2.0 || q.y > H - 1.0 / i.dims.z * 2.0)) {
    rgb = mix(rgb, vec3f(0.0), 0.18);
  }
  if (inBadge) {
    let bd = length(q - bc);
    rgb = mix(u.badge.rgb, vec3f(1.0), smoothstep(br - 1.5, br - 0.5, bd));
    alpha = 1.0;
    let n = i.misc.y;
    let gh = 10.0;
    let len = f32(numberLength(n, 0)) * u.glyph.x * gh;
    let cov = numberCover(q, bc.x + len * 0.5, bc.y - gh * 0.5, gh, n, 0.0, false, 0);
    rgb = mix(rgb, vec3f(1.0), cov);
    return vec4f(themed(rgb, 1.0), alpha * i.dims.w);
  }
  var cov = 0.0;
  var tint = ink.rgb;
  let headH = select(0.0, u.lay.y, u.header.x + u.header.y > 0.5);
  if (q.y < pad + headH) {
    // the header: its icon, then its text
    var x = pad;
    if (u.header.x > 0.5 && i.misc.x >= 0.0) {
      let s = headH - 2.0;
      let t = (q - vec2f(x, pad + 1.0)) / s;
      if (t.x >= 0.0 && t.y >= 0.0 && t.x <= 1.0 && t.y <= 1.0) {
        let col = i.misc.x % u.header.z;
        let row = floor(i.misc.x / u.header.z);
        let ic = textureSampleLevel(icons, smp, (vec2f(col, row) + t) / vec2f(u.header.z, u.header.w), 0.0);
        rgb = mix(rgb, ic.rgb / max(ic.a, 0.0001), ic.a);
      }
      x = x + s + 4.0;
    }
    if (u.header.y > 0.5) {
      let hh = min(i.headSize.y, headH);
      cov = pictureCover(q, i.head, x, pad + (headH - hh) * 0.5, min(i.headSize.x, W - pad - x), hh);
    }
  } else {
    // a row: its label, its value (right), its bar under them
    let k = i32(floor((q.y - pad - headH) / u.lay.z));
    if (k >= 0 && k < i32(u.lay.w) && u.clock.y < 0.5) {
      let y0 = pad + headH + f32(k) * u.lay.z;
      let rw = u.rows[k];
      let rv = rowValue(i, k);
      let hasBar = rw.w > 0.5;
      let lh = u.labelSize[k].y;
      let ly = y0 + (u.lay.z - select(0.0, 4.0, hasBar) - lh) * 0.5;
      let lc = pictureCover(q, u.labels[k], pad, ly, u.labelSize[k].x, lh);
      let gh = 10.0;
      var vc = 0.0;
      if (rw.x < -1.5) {
        // a text row: its item's own picture, right of its label
        let e = i32(i.id + 0.5);
        let slot = i32(rw.y + 0.5);
        let sz = cardItem(e, 4 + 2 * slot);
        vc = pictureCover(q, cardItem(e, 3 + 2 * slot), W - pad - sz.x, y0 + (u.lay.z - select(0.0, 4.0, hasBar) - sz.y) * 0.5, sz.x, sz.y);
      } else {
        vc = numberCover(q, W - pad, y0 + (u.lay.z - select(0.0, 4.0, hasBar) - gh) * 0.5, gh, rv.x, rv.y, rw.y > -0.5, i32(rw.z));
      }
      cov = max(lc, vc);
      tint = select(ink.rgb, mix(ink.rgb, bg.rgb, 0.38), lc > vc);
      if (hasBar) {
        let by = y0 + u.lay.z - 4.5;
        if (q.y >= by && q.y <= by + 3.0 && q.x >= pad && q.x <= W - pad) {
          let lo = u.rowsBar[k].x;
          let hi = select(u.rowsBar[k].y, rv.y, rw.y > -0.5);
          let f = clamp((rv.x - lo) / max(hi - lo, 1e-9), 0.0, 1.0);
          let on = (q.x - pad) / max(W - 2.0 * pad, 1.0) <= f;
          rgb = select(mix(bg.rgb, ink.rgb, 0.15), u.bar.rgb, on);
        }
      }
    }
  }
  rgb = mix(rgb, tint, cov);
  return vec4f(themed(rgb, 1.0), alpha * i.dims.w);
}
`:e===`claim`?`
struct O { @builtin(position) pos: vec4f, @location(0) @interpolate(flat) id: f32 }
@vertex fn vs(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> O {
  let e = i32(iid);
  var o: O;
  o.pos = vec4f(2.0, 2.0, 2.0, 1.0);
  o.id = 0.0;
  let r = cardRoom(e);
  if (r.ok < 0.5 || u.clock.y > 1.5) {
    return o;
  }
  let v = i32(vid);
  let corner = vec2f(select(0.0, 1.0, v == 1 || v == 3), select(0.0, 1.0, v >= 2));
  // (on the claims' picture: a cell a claim.z CSS px, the room grown by 2 px each side, its badge's over its corner)
  let m = 2.0 * frame.size.z;
  let b = select(0.0, 6.0 * r.k * frame.size.z, u.badgeOn.z > 0.5);
  let p = mix(r.box.xy - vec2f(m, m + b), r.box.zw + vec2f(m + b, m), corner) / (u.claim.z * frame.size.z);
  let rank = cardItem(e, 0).w;
  // (in its set's part of the depth, the cards shown nearer: a winner keeps its room; then the one order)
  o.id = cardOrder(e);
  let band = select(0.0, 0.5, cardLater(e));
  o.pos = vec4f(p.x / u.claim.x * 2.0 - 1.0, 1.0 - p.y / u.claim.y * 2.0, (u.channels.z + band + 0.5 * (rank + 1.0) / (u.clock.w + 2.0)) / u.channels.w, 1.0);
  return o;
}
@fragment fn fs(i: O) -> @location(0) vec4f {
  let n = u32(i.id);
  return vec4f(f32(n & 255u), f32((n >> 8u) & 255u), f32((n >> 16u) & 255u), 255.0) / 255.0;
}
`:e===`decide`?`
fn claimOf(x: i32, y: i32) -> i32 {
  let b = round(textureLoad(claims, vec2i(x, y), 0) * 255.0);
  return i32(b.x + b.y * 256.0 + b.z * 65536.0);
}
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let q = vec2i(pos.xy);
  let e = q.y * 4096 + q.x;
  let prev = textureLoad(state, vec2i(e % 4096, e / 4096), 0);
  // (past its items: the decisions of items gone a moment, kept as they are)
  if (e >= i32(u.clock.w)) {
    return prev;
  }
  let fresh = prev.w > 0.99 || u.hold.w > 0.5;
  var shown = stateShown(prev) && !fresh;
  var fade = stateFade(prev);
  let b = round(prev.yz * 255.0);
  var at = b.x * 256.0 + b.y;
  var cov = round(prev.w * 255.0);
  let now = u.hold.x;
  let r = cardRoom(e);
  let placed = r.ok > 0.5 && u.clock.y < 1.5;
  var want = shown;
  var looked = false;
  if (placed) {
    let s = u.claim.z * frame.size.z;
    let off = r.box.z < 0.0 || r.box.x > frame.size.x || r.box.w < 0.0 || r.box.y > frame.size.y;
    if (!off) {
      looked = true;
      // the cells whose middles are inside its room, and those claimed by a card before it in the one order (last
      // frame's: a cell it moved into is empty, or a later card's, and still its)
      let mine = i32(cardOrder(e));
      let c0 = vec2i(ceil(r.box.xy / s - vec2f(0.5)));
      let c1 = vec2i(floor(r.box.zw / s - vec2f(0.5)));
      var cells = 0.0;
      var covered = 0.0;
      for (var y = 0; y < 48; y++) {
        let cy = c0.y + y;
        if (cy > c1.y || (!shown && covered > 0.0)) {
          break;
        }
        for (var x = 0; x < 64; x++) {
          let cx = c0.x + x;
          if (cx > c1.x) {
            break;
          }
          if (cx >= 0 && cy >= 0 && cx < i32(u.claim.x) && cy < i32(u.claim.y)) {
            cells = cells + 1.0;
            let c = claimOf(cx, cy);
            if (c > 0 && c < mine) {
              covered = covered + 1.0;
            }
          }
        }
      }
      if (shown) {
        if (covered > u.room.x * max(cells, 1.0)) {
          let t10 = floor(now / 10.0) % 254.0 + 1.0;
          if (cov < 0.5 || cov > 254.5) {
            cov = t10;
          } else if (((t10 - cov + 254.0) % 254.0) * 10.0 >= u.room.y * 1000.0) {
            want = false;
          }
        } else {
          cov = 0.0;
        }
      } else {
        want = covered < 0.5;
      }
    }
  }
  let since = (now - at + 63500.0) % 63500.0;
  // (not decided yet: at once where its room was looked at; off the screen it waits, not decided)
  if (fresh && looked) {
    shown = want;
    at = now;
    cov = 0.0;
  } else if (fresh) {
    shown = false;
    cov = 255.0;
  } else if (want != shown && placed) {
    if (!want) {
      shown = false;
      at = now;
      cov = 0.0;
    } else if (since >= u.room.z * 1000.0) {
      shown = true;
      at = now;
      cov = 0.0;
    }
  }
  // (its thing not placed: hidden, and decided at once when it is placed again)
  if (!placed) {
    shown = false;
    cov = 255.0;
  }
  fade = clamp(fade + select(-1.0, 1.0, shown) * u.hold.y / max(u.room.w, 0.001), 0.0, 1.0);
  return vec4f((select(0.0, 128.0, shown) + round(fade * 127.0)) / 255.0, floor(at / 256.0) / 255.0, (at % 256.0) / 255.0, cov / 255.0);
}
`:`
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
  let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
  return vec4f(p * 2.0 - 1.0, 0.0, 1.0);
}
@fragment fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
  let q = vec2i(pos.xy);
  let e = q.y * 4096 + q.x;
  if (e >= i32(u.more.y)) {
    return vec4f(0.0, 0.0, 0.0, 1.0);
  }
  let old = i32(cardItem(e, 2).z + 0.5) - 1;
  if (old < 0) {
    return vec4f(0.0, 0.0, 0.0, 1.0);
  }
  return textureLoad(state, vec2i(old % 4096, old / 4096), 0);
}
`}`}const J_=[`items`,`times`,`values`,`texts`,`glyphs`,`icons`,`fills`,`state`,`claims`],Y_=/* @__PURE__ */ new Set([`texts`,`glyphs`,`icons`]);function X_(e,t){let n=`${t?Is(t):mt}
layout(std140) uniform U { vec4 place; vec4 size; vec4 lay; vec4 clock; vec4 look; vec4 background; vec4 room; vec4 more; vec4 bar; vec4 dark; vec4 light; vec4 badge; vec4 fill; vec4 badgeOn; vec4 glyph; vec4 claim; vec4 hold; vec4 header; vec4 channels; vec4 rows[6]; vec4 rowsBar[6]; vec4 labels[6]; vec4 labelSize[6]; } u;
${t?``:`uniform highp sampler2D places;`}
${J_.map(e=>`uniform highp sampler2D ${e};`).join(`
`)}
${t?ws:Cs}
vec4 cardItem(int e, int k) {
  int i = e * int(u.more.x) + k;
  return texelFetch(items, ivec2(i % 4096, i / 4096), 0);
}
float cardWide(int e) {
  float w = cardItem(e, 2).w;
  return w > 0.5 ? w : 120.0;
}
float stateFade(vec4 s) {
  float r = round(s.x * 255.0);
  return mod(r, 128.0) / 127.0;
}
bool stateShown(vec4 s) {
  return round(s.x * 255.0) >= 128.0;
}
bool cardLater(int e) {
  vec4 s = texelFetch(state, ivec2(e % 4096, e / 4096), 0);
  return !(stateShown(s) && s.w < 0.99);
}
float cardOrder(int e) {
  return u.glyph.w + cardItem(e, 0).w + (cardLater(e) ? 262144.0 : 0.0);
}
float cardTime(int sl) {
  return texelFetch(times, ivec2(sl % 4096, sl / 4096), 0).x;
}
vec3 cardSample(int e) {
  vec4 m = cardItem(e, 0);
  int a = int(m.x);
  int n = int(m.y);
  float t = u.clock.x;
  if (n <= 1 || t <= cardTime(a)) {
    return vec3(float(a), float(a), 0.0);
  }
  if (t >= cardTime(a + n - 1)) {
    return vec3(float(a + n - 1), float(a + n - 1), 0.0);
  }
  int lo = 0;
  int hi = n - 1;
  for (int k = 0; k < 24; k++) {
    if (hi - lo <= 1) {
      break;
    }
    int mid = (lo + hi) / 2;
    if (cardTime(a + mid) <= t) {
      lo = mid;
    } else {
      hi = mid;
    }
  }
  float ta = cardTime(a + lo);
  float tb = cardTime(a + hi);
  return vec3(float(a + lo), float(a + hi), tb > ta ? clamp((t - ta) / (tb - ta), 0.0, 1.0) : 0.0);
}
float cardValueAt(int sl, int c) {
  int Q = int(u.channels.x);
  int i = sl * Q + c / 4;
  vec4 v = texelFetch(values, ivec2(i % 4096, i / 4096), 0);
  int k = c % 4;
  return k == 0 ? v.x : k == 1 ? v.y : k == 2 ? v.z : v.w;
}
float cardChannel(vec3 sm, float c) {
  if (c < -0.5) {
    return 0.0;
  }
  int ci = int(c + 0.5);
  return mix(cardValueAt(int(sm.x), ci), cardValueAt(int(sm.y), ci), sm.z);
}
float cardHeight() {
  float st = u.clock.y;
  float head = u.header.x + u.header.y > 0.5 ? u.lay.y : 0.0;
  float body = st < 0.5 ? u.lay.z * u.lay.w : 0.0;
  return 2.0 * u.lay.x + head + body;
}
struct Room { vec4 box; float depth; float ok; float k; };
Room cardRoom(int e) {
  Room r;
  r.box = vec4(0.0);
  r.depth = 0.0;
  r.ok = 0.0;
  r.k = 1.0;
  vec4 a = anchorOf(e);
  if (a.w < 0.5 || anchorBehind(a.xyz)) {
    return r;
  }
  vec4 clip = frame.viewProj * vec4(a.xyz, 1.0);
  if (clip.w < 0.01) {
    return r;
  }
  vec3 ndc = clip.xyz / clip.w;
  vec2 px = vec2(round((ndc.x * 0.5 + 0.5) * frame.size.x), round((0.5 - ndc.y * 0.5) * frame.size.y));
  float w = sizePx(u.size.w, vec4(u.size.xyz, 0.0), a.xyz);
  float k = w / 120.0;
  float dpr = frame.size.z;
  float W = round(cardWide(e) * k * dpr);
  float H = round(cardHeight() * k * dpr);
  float y1 = px.y - round((u.look.y + 2.0) * k * dpr);
  float x0 = px.x - round(W * 0.5);
  r.box = vec4(x0, y1 - H, x0 + W, y1);
  r.depth = clamp(ndc.z, 0.0, 1.0);
  r.ok = 1.0;
  r.k = k;
  return r;
}
vec4 cardClip(vec2 p, float z) {
  return clipOut(vec4(p.x / frame.size.x * 2.0 - 1.0, 1.0 - p.y / frame.size.y * 2.0, z, 1.0));
}
`;return e===`cards`?{vertex:n+`
out vec2 vLocal;
flat out vec4 vDims;
flat out vec4 vRa;
flat out vec4 vRb;
flat out vec4 vRc;
flat out vec4 vFill;
flat out vec4 vMisc;
flat out vec4 vHead;
flat out vec4 vHeadSize;
flat out float vId;
vec2 cardCorner(int v) {
  float x = (v == 1 || v == 3) ? 1.0 : 0.0;
  float y = v >= 2 ? 1.0 : 0.0;
  return vec2(x, y);
}
vec4 cardRamp(float v) {
  float t = clamp((v - u.fill.y) / max(u.fill.z - u.fill.y, 1e-9), 0.0, 1.0);
  return texelFetch(fills, ivec2(int(t * 255.0 + 0.5), 0), 0);
}
float cardShown(int e) {
  if (u.claim.w < 0.5) {
    return 1.0;
  }
  return stateFade(texelFetch(state, ivec2(e % 4096, e / 4096), 0));
}
void main() {
  int e = gl_InstanceID;
  bool isDot = gl_VertexID >= 4;
  vec2 corner = cardCorner(gl_VertexID % 4);
  gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  vLocal = vec2(0.0);
  vDims = vec4(0.0);
  vRa = vec4(0.0);
  vRb = vec4(0.0);
  vRc = vec4(0.0);
  vFill = vec4(0.0);
  vMisc = vec4(0.0);
  vHead = vec4(0.0);
  vHeadSize = vec4(0.0);
  vId = float(e);
  Room r = cardRoom(e);
  if (r.ok < 0.5) {
    return;
  }
  float seen = cardShown(e);
  vec3 sm = cardSample(e);
  float fv = cardChannel(sm, u.fill.x);
  vec4 fc = u.fill.w > 0.5 ? cardRamp(fv) : u.background;
  float dpr = frame.size.z;
  vec2 tip = vec2((r.box.x + r.box.z) * 0.5, r.box.w + round((u.look.y + 2.0) * r.k * dpr));
  if (isDot) {
    float lost = u.clock.y > 1.5 ? 1.0 : 1.0 - cardShown(e);
    if (lost * u.look.z <= 0.0) {
      return;
    }
    float d = round(u.look.z * dpr);
    vec2 p = tip + (corner - vec2(0.5)) * d;
    gl_Position = cardClip(p, r.depth);
    vLocal = (corner - vec2(0.5)) * 2.0;
    vDims = vec4(-1.0, d, 0.0, lost * u.look.x);
    vFill = u.fill.w > 0.5 ? fc : u.dark;
    return;
  }
  if (u.clock.y > 1.5 || seen <= 0.0) {
    return;
  }
  float m = round(8.0 * r.k * dpr);
  vec4 box = vec4(r.box.x, r.box.y - m, r.box.z + m, tip.y);
  vec2 p = mix(box.xy, box.zw, corner);
  gl_Position = cardClip(p, r.depth);
  vLocal = (p - r.box.xy) / dpr / r.k;
  vDims = vec4((r.box.z - r.box.x) / dpr / r.k, (r.box.w - r.box.y) / dpr / r.k, r.k * dpr, seen * u.look.x);
  vRa = vec4(cardChannel(sm, u.rows[0].x), cardChannel(sm, u.rows[0].y), cardChannel(sm, u.rows[1].x), cardChannel(sm, u.rows[1].y));
  vRb = vec4(cardChannel(sm, u.rows[2].x), cardChannel(sm, u.rows[2].y), cardChannel(sm, u.rows[3].x), cardChannel(sm, u.rows[3].y));
  vRc = vec4(cardChannel(sm, u.rows[4].x), cardChannel(sm, u.rows[4].y), cardChannel(sm, u.rows[5].x), cardChannel(sm, u.rows[5].y));
  vFill = fc;
  vec4 it = cardItem(e, 0);
  vMisc = vec4(it.z, cardChannel(sm, u.badgeOn.x), (cardChannel(sm, u.badgeOn.y) > 0.5 && u.badgeOn.z > 0.5) ? 1.0 : 0.0, 0.0);
  vHead = cardItem(e, 1);
  vHeadSize = cardItem(e, 2);
}
`,fragment:n+`
in vec2 vLocal;
flat in vec4 vDims;
flat in vec4 vRa;
flat in vec4 vRb;
flat in vec4 vRc;
flat in vec4 vFill;
flat in vec4 vMisc;
flat in vec4 vHead;
flat in vec4 vHeadSize;
flat in float vId;
out vec4 outColor;
float glyphCover(int code, float x, float y) {
  if (code < 0 || x < 0.0 || x > u.glyph.y || y < 0.0 || y > 1.0) {
    return 0.0;
  }
  vec2 uv = vec2((float(code) * u.glyph.y + x) / (u.glyph.z * u.glyph.y), y);
  return textureLod(glyphs, uv, 0.0).a;
}
int digitCount(int n) {
  int c = 1;
  int m = n / 10;
  for (int k = 0; k < 9; k++) {
    if (m <= 0) {
      break;
    }
    c++;
    m = m / 10;
  }
  return c;
}
int powTen(int d) {
  return d >= 2 ? 100 : d == 1 ? 10 : 1;
}
int numberChar(float v, int dec, int r) {
  int p = powTen(dec);
  int iv = int(round(min(abs(v), 9999999.0) * float(p)));
  int ip = iv / p;
  int fp = iv - ip * p;
  if (r < dec) {
    return (fp / powTen(r)) % 10;
  }
  int k = r - dec;
  if (dec > 0) {
    if (k == 0) {
      return 10;
    }
    k = k - 1;
  }
  int n = digitCount(ip);
  if (k < n) {
    int q = ip;
    for (int j = 0; j < 9; j++) {
      if (j >= k) {
        break;
      }
      q = q / 10;
    }
    return q % 10;
  }
  if (k == n && v < 0.0 && iv > 0) {
    return 11;
  }
  return -1;
}
int numberLength(float v, int dec) {
  int p = powTen(dec);
  int iv = int(round(min(abs(v), 9999999.0) * float(p)));
  return digitCount(iv / p) + (dec > 0 ? dec + 1 : 0) + ((v < 0.0 && iv > 0) ? 1 : 0);
}
float numberCover(vec2 q, float x1, float y0, float h, float v, float ofv, bool hasOf, int dec) {
  float adv = u.glyph.x * h;
  int r = int(floor((x1 - q.x) / adv));
  if (r < 0 || q.y < y0 || q.y > y0 + h) {
    return 0.0;
  }
  int code = -1;
  int rr = r;
  if (hasOf) {
    int no = numberLength(ofv, 0);
    if (rr < no) {
      code = numberChar(ofv, 0, rr);
    } else if (rr == no) {
      code = 12;
    }
    rr = rr - no - 1;
  }
  if (code < 0 && rr >= 0) {
    code = numberChar(v, dec, rr);
  }
  float x = (x1 - float(r + 1) * adv - (u.glyph.y * h - adv) * 0.5);
  return glyphCover(code, (q.x - x) / h, (q.y - y0) / h);
}
float pictureCover(vec2 q, vec4 rect, float x0, float y0, float w, float h) {
  vec2 t = (q - vec2(x0, y0)) / vec2(w, h);
  if (t.x < 0.0 || t.y < 0.0 || t.x > 1.0 || t.y > 1.0) {
    return 0.0;
  }
  return textureLod(texts, mix(rect.xy, rect.zw, t), 0.0).a;
}
vec2 rowValue(int k) {
  if (k < 2) {
    return k == 0 ? vRa.xy : vRa.zw;
  }
  if (k < 4) {
    return k == 2 ? vRb.xy : vRb.zw;
  }
  return k == 4 ? vRc.xy : vRc.zw;
}
void main() {
  vec2 q = vLocal;
  float W = vDims.x;
  if (vDims.x < -0.5) {
    float r = length(q);
    float px = 2.0 / max(vDims.y, 1.0);
    float disc = 1.0 - smoothstep(1.0 - px, 1.0, r);
    float rim = smoothstep(1.0 - px * 4.0, 1.0 - px * 3.0, r);
    float a = disc * vDims.w;
    if (a <= 0.004) {
      discard;
    }
    if (frame.pick.x > 0.5) {
      outColor = pickOut(u.clock.z, vId);
      return;
    }
    outColor = vec4(themed(mix(vFill.rgb, u.dark.rgb, rim * 0.85), 1.0), a);
    return;
  }
  float pad = u.lay.x;
  float H = cardHeight();
  float rad = u.look.w;
  vec2 c = clamp(q, vec2(rad), vec2(W - rad, H - rad));
  bool inBox = length(q - c) <= rad;
  float pt = u.look.y;
  bool inTip = q.y >= H - 0.5 && q.y <= H + pt && abs(q.x - W * 0.5) <= (H + pt - q.y) * 0.9;
  vec2 bc = vec2(W - 3.0, 3.0);
  float br = 8.0;
  bool inBadge = vMisc.z > 0.5 && length(q - bc) <= br;
  if (!inBox && !inTip && !inBadge) {
    discard;
  }
  if (frame.pick.x > 0.5) {
    outColor = pickOut(u.clock.z, vId);
    return;
  }
  vec4 bg = vFill;
  float lum = dot(bg.rgb, vec3(0.299, 0.587, 0.114));
  vec4 ink = lum > 0.55 ? u.dark : u.light;
  vec3 rgb = bg.rgb;
  float alpha = bg.a;
  float edge = length(q - c) - rad;
  if (inBox && (edge > -1.0 / vDims.z * 2.0 || q.y > H - 1.0 / vDims.z * 2.0)) {
    rgb = mix(rgb, vec3(0.0), 0.18);
  }
  if (inBadge) {
    float bd = length(q - bc);
    rgb = mix(u.badge.rgb, vec3(1.0), smoothstep(br - 1.5, br - 0.5, bd));
    alpha = 1.0;
    float n = vMisc.y;
    float gh = 10.0;
    float len = float(numberLength(n, 0)) * u.glyph.x * gh;
    float cov = numberCover(q, bc.x + len * 0.5, bc.y - gh * 0.5, gh, n, 0.0, false, 0);
    rgb = mix(rgb, vec3(1.0), cov);
    outColor = vec4(themed(rgb, 1.0), alpha * vDims.w);
    return;
  }
  float cov = 0.0;
  vec3 tint = ink.rgb;
  float headH = u.header.x + u.header.y > 0.5 ? u.lay.y : 0.0;
  if (q.y < pad + headH) {
    float x = pad;
    if (u.header.x > 0.5 && vMisc.x >= 0.0) {
      float s = headH - 2.0;
      vec2 t = (q - vec2(x, pad + 1.0)) / s;
      if (t.x >= 0.0 && t.y >= 0.0 && t.x <= 1.0 && t.y <= 1.0) {
        float col = mod(vMisc.x, u.header.z);
        float row = floor(vMisc.x / u.header.z);
        vec4 ic = textureLod(icons, (vec2(col, row) + t) / vec2(u.header.z, u.header.w), 0.0);
        rgb = mix(rgb, ic.rgb / max(ic.a, 0.0001), ic.a);
      }
      x = x + s + 4.0;
    }
    if (u.header.y > 0.5) {
      float hh = min(vHeadSize.y, headH);
      cov = pictureCover(q, vHead, x, pad + (headH - hh) * 0.5, min(vHeadSize.x, W - pad - x), hh);
    }
  } else {
    int k = int(floor((q.y - pad - headH) / u.lay.z));
    if (k >= 0 && k < int(u.lay.w) && u.clock.y < 0.5) {
      float y0 = pad + headH + float(k) * u.lay.z;
      vec4 rw = u.rows[k];
      vec2 rv = rowValue(k);
      bool hasBar = rw.w > 0.5;
      float lh = u.labelSize[k].y;
      float ly = y0 + (u.lay.z - (hasBar ? 4.0 : 0.0) - lh) * 0.5;
      float lc = pictureCover(q, u.labels[k], pad, ly, u.labelSize[k].x, lh);
      float gh = 10.0;
      float vc = 0.0;
      if (rw.x < -1.5) {
        int e = int(vId + 0.5);
        int slot = int(rw.y + 0.5);
        vec4 sz = cardItem(e, 4 + 2 * slot);
        vc = pictureCover(q, cardItem(e, 3 + 2 * slot), W - pad - sz.x, y0 + (u.lay.z - (hasBar ? 4.0 : 0.0) - sz.y) * 0.5, sz.x, sz.y);
      } else {
        vc = numberCover(q, W - pad, y0 + (u.lay.z - (hasBar ? 4.0 : 0.0) - gh) * 0.5, gh, rv.x, rv.y, rw.y > -0.5, int(rw.z));
      }
      cov = max(lc, vc);
      tint = lc > vc ? mix(ink.rgb, bg.rgb, 0.38) : ink.rgb;
      if (hasBar) {
        float by = y0 + u.lay.z - 4.5;
        if (q.y >= by && q.y <= by + 3.0 && q.x >= pad && q.x <= W - pad) {
          float lo = u.rowsBar[k].x;
          float hi = rw.y > -0.5 ? rv.y : u.rowsBar[k].y;
          float f = clamp((rv.x - lo) / max(hi - lo, 1e-9), 0.0, 1.0);
          bool on = (q.x - pad) / max(W - 2.0 * pad, 1.0) <= f;
          rgb = on ? u.bar.rgb : mix(bg.rgb, ink.rgb, 0.15);
        }
      }
    }
  }
  rgb = mix(rgb, tint, cov);
  outColor = vec4(themed(rgb, 1.0), alpha * vDims.w);
}
`}:e===`claim`?{vertex:n+`
flat out float vId;
void main() {
  int e = gl_InstanceID;
  gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
  vId = 0.0;
  Room r = cardRoom(e);
  if (r.ok < 0.5 || u.clock.y > 1.5) {
    return;
  }
  int v = gl_VertexID;
  vec2 corner = vec2((v == 1 || v == 3) ? 1.0 : 0.0, v >= 2 ? 1.0 : 0.0);
  float m = 2.0 * frame.size.z;
  float b = u.badgeOn.z > 0.5 ? 6.0 * r.k * frame.size.z : 0.0;
  vec2 p = mix(r.box.xy - vec2(m, m + b), r.box.zw + vec2(m + b, m), corner) / (u.claim.z * frame.size.z);
  float rank = cardItem(e, 0).w;
  vId = cardOrder(e);
  float band = cardLater(e) ? 0.5 : 0.0;
  gl_Position = vec4(p.x / u.claim.x * 2.0 - 1.0, 1.0 - p.y / u.claim.y * 2.0, ((u.channels.z + band + 0.5 * (rank + 1.0) / (u.clock.w + 2.0)) / u.channels.w) * 2.0 - 1.0, 1.0);
}
`,fragment:n+`
flat in float vId;
out vec4 outColor;
void main() {
  uint n = uint(vId);
  outColor = vec4(float(n & 255u), float((n >> 8u) & 255u), float((n >> 16u) & 255u), 255.0) / 255.0;
}
`}:{vertex:n+`
void main() {
  vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
  gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}
`,fragment:n+(e===`decide`?`
out vec4 outColor;
// (GL's rows from the bottom: the caller turns a row from the top over)
int claimOf(int x, int y) {
  vec4 b = round(texelFetch(claims, ivec2(x, y), 0) * 255.0);
  return int(b.x + b.y * 256.0 + b.z * 65536.0);
}
void main() {
  ivec2 q = ivec2(gl_FragCoord.xy);
  int e = q.y * 4096 + q.x;
  vec4 prev = texelFetch(state, ivec2(e % 4096, e / 4096), 0);
  if (e >= int(u.clock.w)) {
    outColor = prev;
    return;
  }
  bool fresh = prev.w > 0.99 || u.hold.w > 0.5;
  bool shown = stateShown(prev) && !fresh;
  float fade = stateFade(prev);
  vec2 b = round(prev.yz * 255.0);
  float at = b.x * 256.0 + b.y;
  float cov = round(prev.w * 255.0);
  float now = u.hold.x;
  Room r = cardRoom(e);
  bool placed = r.ok > 0.5 && u.clock.y < 1.5;
  bool want = shown;
  bool looked = false;
  if (placed) {
    float s = u.claim.z * frame.size.z;
    bool off = r.box.z < 0.0 || r.box.x > frame.size.x || r.box.w < 0.0 || r.box.y > frame.size.y;
    if (!off) {
      looked = true;
      int mine = int(cardOrder(e));
      ivec2 c0 = ivec2(ceil(r.box.xy / s - vec2(0.5)));
      ivec2 c1 = ivec2(floor(r.box.zw / s - vec2(0.5)));
      float cells = 0.0;
      float covered = 0.0;
      for (int y = 0; y < 48; y++) {
        int cy = c0.y + y;
        if (cy > c1.y || (!shown && covered > 0.0)) {
          break;
        }
        for (int x = 0; x < 64; x++) {
          int cx = c0.x + x;
          if (cx > c1.x) {
            break;
          }
          if (cx >= 0 && cy >= 0 && cx < int(u.claim.x) && cy < int(u.claim.y)) {
            cells = cells + 1.0;
            int c = claimOf(cx, int(u.claim.y) - 1 - cy);
            if (c > 0 && c < mine) {
              covered = covered + 1.0;
            }
          }
        }
      }
      if (shown) {
        if (covered > u.room.x * max(cells, 1.0)) {
          float t10 = mod(floor(now / 10.0), 254.0) + 1.0;
          if (cov < 0.5 || cov > 254.5) {
            cov = t10;
          } else if (mod(t10 - cov + 254.0, 254.0) * 10.0 >= u.room.y * 1000.0) {
            want = false;
          }
        } else {
          cov = 0.0;
        }
      } else {
        want = covered < 0.5;
      }
    }
  }
  float since = mod(now - at + 63500.0, 63500.0);
  if (fresh && looked) {
    shown = want;
    at = now;
    cov = 0.0;
  } else if (fresh) {
    shown = false;
    cov = 255.0;
  } else if (want != shown && placed) {
    if (!want) {
      shown = false;
      at = now;
      cov = 0.0;
    } else if (since >= u.room.z * 1000.0) {
      shown = true;
      at = now;
      cov = 0.0;
    }
  }
  if (!placed) {
    shown = false;
    cov = 255.0;
  }
  fade = clamp(fade + (shown ? 1.0 : -1.0) * u.hold.y / max(u.room.w, 0.001), 0.0, 1.0);
  outColor = vec4(((shown ? 128.0 : 0.0) + round(fade * 127.0)) / 255.0, floor(at / 256.0) / 255.0, mod(at, 256.0) / 255.0, cov / 255.0);
}
`:`
out vec4 outColor;
void main() {
  ivec2 q = ivec2(gl_FragCoord.xy);
  int e = q.y * 4096 + q.x;
  if (e >= int(u.more.y)) {
    outColor = vec4(0.0, 0.0, 0.0, 1.0);
    return;
  }
  int old = int(cardItem(e, 2).z + 0.5) - 1;
  if (old < 0) {
    outColor = vec4(0.0, 0.0, 0.0, 1.0);
    return;
  }
  outColor = texelFetch(state, ivec2(old % 4096, old / 4096), 0);
}
`)}}const Z_=/* @__PURE__ */ new WeakMap,Q_=e=>{let t=Z_.get(e);return t||Z_.set(e,t={pics:[],size:``,cur:0,frame:-1,prev:-1,drawn:!1,sets:[]}),t},$_=10.5,ev=`0123456789.-/%+`,tv=`system-ui, -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif`;var nv=class extends D{kind=`point`;on;g=null;progs={};descs={};progKey=``;built=0;wanted=0;ubuf=null;u=/* @__PURE__ */ new Float32Array(172);quads=null;dots=null;tex={};dummy=null;cols=null;names=[];madeFor=-1;making=!1;logsSeen=``;deriver=null;atlas=null;pics=/* @__PURE__ */ new Map;atlasH=0;atlasGen=-1;itemTex=/* @__PURE__ */ new Float32Array;stride=3;labels=[];glyphM=[.6,.8,15];fillDomain=[0,1];barDomains=[];states=[];cur=0;first=!0;stateRows=0;stateIds=[];carry=!1;carryFrom=null;slotIds=[];gone=/* @__PURE__ */ new Map;stateN=0;slotN=0;decided=!1;room=null;claimedAt=-2;viewAt=-1e9;touchedAt=-1e9;madeAt=-1e9;viewKey=``;step=0;broken=[];constructor(e){if(super(e.order===void 0?{...e,order:6.5}:e),(e.rows?.length??0)>6)throw new K(`E-DATA-001`,`cards '${this.id}': at most six rows (here ${e.rows.length})`,{where:this.id,fix:`six rows, or a second card set`});if((e.rows??[]).forEach((e,t)=>{if(e.value===void 0==(e.text===void 0))throw new K(`E-DATA-001`,`cards '${this.id}': row ${t} ('${e.label}') ${e.value===void 0?`shows nothing`:`has a value and a text`}`,{where:this.id,fix:`{ label, value: 'kw' } (a value) or { label, text: d => d.props.name } (a text of its item's)`})}),e.header?.icon&&!(e.icons instanceof Ec))throw new K(`E-DATA-001`,`cards '${this.id}': a header icon needs the app's pictures`,{where:this.id,fix:`cards({ icons: icons({ station: '/img/station.png' }), header: { icon: 'station' } })`});this.on=new wc(e.of,{at:e.at,lift:e.lift})}async attach(e){await super.attach(e),this.g=e.graphics,this.on.attach(e,()=>{this.madeFor=-1,this.invalidate()}),this.g&&(this.ubuf=this.g.buffer(`uniform`,688,`${this.id} cards`),this.quads=this.g.buffer(`index`,Uint32Array.from([0,1,2,2,1,3]),`${this.id} quad`),this.dots=this.g.buffer(`index`,Uint32Array.from([4,5,6,6,5,7]),`${this.id} dot`),this.dummy=this.g.texture({label:`${this.id} none`,width:1,height:1,format:`rgba8`,filter:`nearest`}),this.dummy.write(/* @__PURE__ */ new Uint8Array(4))),this.props.icons?.load(e.loader).then(()=>this.invalidate(!0)),(this.room=Q_(e.world)).sets.push(this)}invalidate(e=!1){e&&(this.touchedAt=this.attached?.world.now??0),(this.attached?.world)?.invalidate?.()}setProps(e){return super.setProps(e),(`rows`in e||`fill`in e||`badge`in e||`header`in e||`log`in e||`priority`in e||`fit`in e)&&(this.madeFor=-1,this.deriver=null),(`collide`in e||`header`in e)&&(this.progKey=``),this.invalidate(!0),this}logOf(){let e=this.on.set?null:this.props.of;return this.props.log??this.on.set?.log??(e&&e.log instanceof Gs?e.log:null)}channels(){let e=this.props,t={};if((e.rows??[]).forEach((e,n)=>{e.value!==void 0&&(t[`r${n}`]=e.value,e.of!==void 0&&(t[`r${n}of`]=e.of))}),e.fill&&(t.fill=e.fill.value),e.badge){t.badge=e.badge.value;let n=e.badge.when??(e=>e>0);t.badgeOn={formula:(e,t)=>+!!n(e.badge??0,t)}}return t}make(e=!0){let t=this.g,n=this.channels(),r=this.logOf(),i=Cl(n,r,this.on.items);for(let e of Object.values(n))if(Sl(e)&&!e.log&&!r)throw new K(`E-DATA-001`,`cards '${this.id}': '${e.value}' is a log's value, and the cards have no log`,{where:this.id,fix:`cards({ log: history.log({ url }) }) (or give the value its own: { value, log })`});if(i.some(e=>e.loading)){if(!this.making){this.making=!0;let e=this.attached;Promise.all(i.map(t=>t.start({loader:e.loader,error:t=>e.world.error(t)}))).then(()=>{this.making=!1,this.madeFor=-1,this.invalidate()})}return}let a=this.on.items,o=this.on.ids;(!this.deriver||this.deriver.names.join()!==Object.keys(n).join()||this.deriverLog!==r)&&(this.deriver=new Pl(n,{log:r,id:(e,t)=>this.on.ids[t]??t}),this.deriverLog=r,e=!0);let s=this.deriver.derive(a);if(this.logsSeen=this.inputsKey(),!e&&s===this.cols){this.madeFor=this.on.version;return}this.cols=s,this.names=s.values.map(e=>e.name);let c=Math.max(1,this.names.length),l=Math.ceil(c/4),u=s.used,d=Math.max(1,Math.ceil(u/4096)),f=Math.max(1,Math.ceil(u*l/4096)),p=new Float32Array(d*4096),m=new Float32Array(f*4096*4);for(let e=0;e<u;e++){p[e]=s.secAt(e);for(let t=0;t<this.names.length;t++)m[e*l*4+t]=s.cols[t][e]}this.texture(`times`,4096,d,`r32f`,p),this.texture(`values`,4096,f,`rgba32f`,m);let h=e=>{let t=this.names.indexOf(e);if(t<0)return[0,1];let n=1/0,r=-1/0,i=s.cols[t];for(let e=0;e<u;e++){let t=i[e];t<n&&(n=t),t>r&&(r=t)}return Number.isFinite(n)?[n,r>n?r:n+1]:[0,1]},g=this.props.rows??[],_=a.length;if(this.tex.glyphs||this.glyphs(t),e)this.pictures(a,o,h);else{let e=this.itemTex,t=this.stride;for(let n=0;n<_;n++){let r=n*t*4,i=s.indexOf(o[n]??n);e[r]=i>=0?s.first[i]:0,e[r+1]=i>=0?s.count[i]:0}this.texture(`items`,4096,Math.max(1,Math.ceil(t*_/4096)),`rgba32f`,e)}let v=this.props.fill;if(v){let t=v.color.breaks,n=Array.isArray(t)&&t.length===2&&t.every(e=>typeof e==`number`)?t:null,r=v.domain??n??h(`fill`);if(e||!this.tex.fills||r[0]!==this.fillDomain[0]||r[1]!==this.fillDomain[1]){this.fillDomain=r;let e=/* @__PURE__ */ new Uint8Array(1024),[t,n]=r;for(let r=0;r<256;r++){let i=L(v.color(t+(n-t)*r/255));e.set([i[0],i[1],i[2],i[3]].map(e=>Math.round(e*255)),r*4)}this.texture(`fills`,256,1,`rgba8`,e)}}this.barDomains=g.map((e,t)=>typeof e.bar==`object`?e.bar.domain:[0,h(`r${t}`)[1]]),this.madeFor=this.on.version,e&&(this.claimedAt=-2,this.decided=!1,this.madeAt=this.attached?.world.now??0),(this.attached?.world)?.invalidate?.()}deriverLog=null;pictures(e,t,n){let r=this.cols,i=Math.max(2,Math.min(3,typeof devicePixelRatio==`number`?devicePixelRatio:2)),a=this.props.rows??[],o=a.map((e,t)=>e.text===void 0?-1:t).filter(e=>e>=0),s=e.length,c=this.props.fit,l=typeof c==`object`?c:{},u=c?120*(l.min??.5):120,d=c?120*Math.max(l.min??.5,l.max??2):120,f=this.props.header?.icon?20:0,p=d-12-f,m=s*(1+o.length)+a.length;(!this.atlas||this.pics.size>2*m+256)&&(this.atlas=new tn,this.pics.clear());let h=this.atlas,g=this.pics;this.atlasGen!==h.gen&&(g.clear(),this.atlasGen=h.gen);let _=(e,t,n,r)=>{let a=`${e}|${t}|${n}|${r}`,o=g.get(a);if(o)return o;let s=`${n} ${Math.round(t*i)}px ${tv}`,c=h.g;c.font=s;let l=e;if(c.measureText(l).width/i>r){for(;l.length>1&&c.measureText(`${l}…`).width/i>r;)l=l.slice(0,-1);l=`${l}…`}let u=Math.ceil(c.measureText(l).width)+2,d=Math.ceil(t*i*1.3),f={r:h.get(a,u,d,(e,t,n)=>{e.font=s,e.textBaseline=`middle`,e.fillStyle=`#ffffff`,e.fillText(l,t+1,n+d/2)}),w:u/i,h:d/i};return g.set(a,f),f},v=this.props.header?.text,y=()=>{let t=a.map(e=>_(e.label,$_,500,c?Math.max(70,p*.6):70));return{labels:t,heads:e.map(e=>{let t=typeof v==`function`?v(e):v;return t==null||t===``?null:_(String(t),12,650,p)}),rowTexts:o.map(n=>{let r=a[n],i=Math.max(12,d-12-t[n].w-6);return e.map(e=>{let t=r.text(e);return t==null||t===``?null:_(String(t),$_,650,i)})})}},b=h.gen,x=y();h.gen!==b&&(g.clear(),this.atlasGen=h.gen,x=y());let{labels:S,heads:C,rowTexts:w}=x,T=h.size,E=Math.max(1,Math.min(T,Math.ceil(Math.max(1,...[...g.values()].map(e=>e.r.y+e.r.h)))));if(h.dirty||E!==this.atlasH||!this.tex.texts||this.tex.texts.desc.width!==T){let e=h.g.getImageData(0,0,T,E);this.texture(`texts`,T,E,`rgba8`,new Uint8Array(e.data.buffer),`linear`),h.dirty=null,this.atlasH=E}let D=e=>[e.r.x/T,e.r.y/E,(e.r.x+e.r.w)/T,(e.r.y+e.r.h)/E];this.labels=S.map(e=>({rect:D(e),w:e.w,h:e.h}));let O=this.glyphM[0]*10,k=(e,t)=>String(Math.round(Math.abs(e))).length+(t?t+1:0)+ +(e<0),A=a.map((e,t)=>{if(e.text!==void 0)return 0;let[r,i]=n(`r${t}`),a=e.decimals??0,o=e.of===void 0?null:n(`r${t}of`);return O*(Math.max(k(r,a),k(i,a))+(o?1+Math.max(k(o[0],a),k(o[1],a)):0))}),j=this.props.priority,M=e.map((e,n)=>({i:n,p:j&&Number(j(e))||0,id:t[n]??``})).sort((e,t)=>t.p-e.p||(e.id<t.id?-1:+(e.id>t.id))),N=new Float32Array(s);M.forEach((e,t)=>{N[e.i]=t});let P=this.props.header?.icon,F=this.props.icons,I=this.attached?.world.now??0,L=new Set(t),R=this.props.collide===!1?0:this.timing().away*1e3,z=/* @__PURE__ */ new Map,B=this.stateN;this.stateIds.forEach((e,t)=>{(t<B||(this.gone.get(e)??-1/0)>=I)&&z.set(e,t)});for(let e=0;e<Math.min(B,this.stateIds.length);e++){let t=this.stateIds[e];!L.has(t)&&R>0&&this.gone.set(t,I+R)}for(let[e,t]of this.gone)(L.has(e)||t<I||!z.has(e))&&this.gone.delete(e);let V=[...this.gone.keys()],H=s+V.length;this.slotIds=[...t,...V],this.slotN=s;let U=this.stride=3+2*o.length,ee=Math.max(1,Math.ceil(U*H/4096)),W=this.itemTex=new Float32Array(ee*4096*4);V.forEach((e,t)=>{W[(s+t)*U*4+10]=(z.get(e)??-1)+1});for(let n=0;n<s;n++){let i=n*U*4,s=r.indexOf(t[n]??n),l=typeof P==`function`?P(e[n]):P,p=C[n];W[i]=s>=0?r.first[s]:0,W[i+1]=s>=0?r.count[s]:0,W[i+2]=l&&F?F.index(l):-1,W[i+3]=N[n],p&&(W.set(D(p),i+4),W[i+8]=p.w,W[i+9]=p.h),W[i+10]=(z.get(t[n]??``)??-1)+1;let m=p?12+f+p.w:0;a.forEach((e,t)=>{m=Math.max(m,12+S[t].w+8+(e.text===void 0?A[t]:w[o.indexOf(t)][n]?.w??0))}),W[i+11]=c?Math.min(d,Math.max(u,Math.ceil(m))):0,o.forEach((e,t)=>{let r=w[t][n];r&&(W.set(D(r),i+12+t*8),W[i+16+t*8]=r.w,W[i+17+t*8]=r.h)})}this.texture(`items`,4096,ee,`rgba32f`,W),this.carry=this.stateIds.length>0}get slots(){return Math.max(this.on.items.length,this.slotIds.length)}texture(e,t,n,r,i,a=`nearest`){let o=this.tex[e];(!o||o.desc.width!==t||o.desc.height!==n||o.desc.format!==r)&&(o?.destroy(),this.tex[e]=this.g.texture({label:`${this.id} ${e}`,width:t,height:n,format:r,filter:a})),this.tex[e].write(i)}glyphs(e){if(typeof OffscreenCanvas>`u`)return;let t=`700 34px ${tv}`,n=new OffscreenCanvas(8,8).getContext(`2d`);if(!n)return;n.font=t;let r=Math.max(...[...`0123456789`].map(e=>n.measureText(e).width)),i=Math.ceil(Math.max(...[...ev].map(e=>n.measureText(e).width))+4),a=new OffscreenCanvas(i*15,40),o=a.getContext(`2d`,{willReadFrequently:!0});if(!o)return;o.font=t,o.fillStyle=`#ffffff`,o.textBaseline=`middle`,o.textAlign=`center`,[...ev].forEach((e,t)=>o.fillText(e,t*i+i/2,21));let s=new Uint8Array(o.getImageData(0,0,a.width,40).data.buffer),c=e.texture({label:`${this.id} glyphs`,width:a.width,height:40,format:`rgba8`,filter:`linear`});c.write(s),this.tex.glyphs=c,this.glyphM=[r/40,i/40,15]}programs(e){let t=this.props.collide!==!1,n=`${JSON.stringify(e)}|${t}`;if(n===this.progKey)return;this.progKey=n;for(let e of Object.values(this.progs))e?.destroy();this.progs={},this.built=0,this.wanted=0,this.broken=[];let r=this.g,i=[...(e=>e.map(e=>({name:e,kind:`2d`,sample:`unfilterable`})))(e?ss:[`places`]),...J_.map(e=>({name:e,kind:`2d`,sample:Y_.has(e)?`float`:`unfilterable`}))],a=e?[`Frame`,`S`,`U`]:[`Frame`,`U`],o=(t,n)=>{let o={label:`cards ${this.id} ${t}`,wgsl:q_(t,e),glsl:X_(t,e),blocks:a,textures:i,...n},s=r.program(o);return this.wanted++,s.ready.then(()=>{this.built++,this.invalidate()},e=>{this.broken.push(`${t}: ${e?.message??e}`),this.attached?.world.error(e)}),{p:s,desc:o}},s=o(`cards`,{blend:`alpha`,depth:{write:!1,test:`always`}});this.progs.cards=s.p,this.descs.cards=s.desc,t&&(this.progs.claim=o(`claim`,{blend:`none`,depth:{write:!0,test:`less`}}).p,this.progs.decide=o(`decide`,{overlay:!0,blend:`none`}).p,this.progs.carry=o(`carry`,{overlay:!0,blend:`none`}).p),this.first=!0,this.decided=!1}freshStates(e,t){let n=new Uint8Array(4096*t*4);for(let e=3;e<n.length;e+=4)n[e]=255;return[0,1].map(r=>{let i=e.texture({label:`${this.id} decisions ${r}`,width:4096,height:t,format:`rgba8`,filter:`nearest`,renderable:!0});return i.write(n),{tex:i,target:e.layerTarget(i,0)}})}collision(e,t){let n=e.graphics,r=Math.ceil(e.camera.width/4),i=Math.ceil(e.camera.height/4),a=this.room;if(a.size!==`${r}x${i}`){for(let e of a.pics)e.destroy();a.pics=[0,1].map(e=>n.target(r,i,{sampled:!0,label:`cards room ${e}`})),a.size=`${r}x${i}`,a.frame=-1,a.prev=-1}a.frame!==e.now&&(a.prev=a.frame,a.frame=e.now,a.cur=1-a.cur,a.drawn=!1);let o=Math.max(1,Math.ceil(Math.max(t,this.slots)/4096));if(this.stateRows!==o){let e=this.states,t=e=>{for(let t of e)t.target.destroy(),t.tex.destroy()};this.carry&&e.length?this.carryFrom?t(e):this.carryFrom=[e[this.cur],e[1-this.cur]]:(t(e),this.carryFrom&&t(this.carryFrom),this.carryFrom=null,this.stateIds=[],this.stateN=0,this.carry=!1,this.gone.clear()),this.states=this.freshStates(n,o),this.stateRows=o,this.cur=0}}frame(e){let t=e.graphics,n=this.props;if(!this.ubuf||!this.on.frame(e)||(this.madeFor===this.on.version?this.cols&&this.logsChanged(e)&&this.make(!1):this.make(!0),!this.cols||!this.tex.items||!this.tex.glyphs))return[];let r=this.on.set?this.on.track(e)?.variant??null:null;if(this.on.set&&!r||(this.programs(r),this.built<this.wanted))return[];let i=this.on.items.length;if(!i)return[];let a=n.collide!==!1;a&&this.collision(e,i);let o=e.camera,s=`${o.viewProj.slice(0,16).join(`,`)}|${o.width}x${o.height}`;s!==this.viewKey&&(this.viewKey=s,this.viewAt=e.now),this.uniforms(e,i,a);let c=this.dummy,l=(r,i)=>[...this.on.textures(e),this.tex.items,this.tex.times,this.tex.values,this.tex.texts,this.tex.glyphs,n.icons?.texture(t)??c,this.tex.fills??c,r??c,i??c],u=[...this.on.blocks(e),this.ubuf],d=this.room;if(a&&this.progs.claim&&this.progs.decide&&this.progs.carry&&d.pics.length){if(this.carry){let e=this.carryFrom?this.carryFrom[0]:this.states[this.cur],n=this.carryFrom?this.states[this.cur]:this.states[1-this.cur];if(t.frame([{program:this.progs.carry,blocks:u,textures:l(e.tex,null),count:3}],{target:n.target,clear:[0,0,0,1]}),this.carryFrom){for(let e of this.carryFrom)e.target.destroy(),e.tex.destroy();this.carryFrom=null}else this.cur=1-this.cur;this.carry=!1}this.slotIds.length>=this.on.ids.length?(this.stateIds=this.slotIds,this.stateN=this.slotN):(this.stateIds=this.on.ids.slice(),this.stateN=this.stateIds.length);let e=this.states[this.cur],n=this.states[1-this.cur],r=d.pics[d.cur],a=d.pics[1-d.cur];this.claimedAt===d.prev&&a.texture&&(t.frame([{program:this.progs.decide,blocks:u,textures:l(e.tex,a.texture),count:3}],{target:n.target,clear:[0,0,0,0]}),this.cur=1-this.cur,this.decided=!0,this.first=!1),t.frame([{program:this.progs.claim,blocks:u,textures:l(this.states[this.cur].tex,null),index:this.quads,count:6,instanceCount:i}],{target:r,clear:[0,0,0,0],keep:d.drawn,keepDepth:!0}),d.drawn=!0,this.claimedAt=d.frame}let f=a?this.states[this.cur]?.tex??null:null,p=this.descs.cards,m=n.pickable===!1?void 0:()=>this.progs.pick??=t.program({...p,label:`${p.label} pick`,blend:`none`}),h=[];return n.look?.dots!==!1&&(a||this.step>1)&&h.push({program:this.progs.cards,blocks:u,textures:l(f,null),index:this.dots,count:6,instanceCount:i,pickProgram:m}),h.push({program:this.progs.cards,blocks:u,textures:l(f,null),index:this.quads,count:6,instanceCount:i,pickProgram:m}),h}logsAt=-1e9;logsChanged(e){return e.now-this.logsAt<500&&e.now>=this.logsAt?!1:(this.logsAt=e.now,this.inputsKey()!==this.logsSeen)}timing(){let e=typeof this.props.collide==`object`?this.props.collide:{};return{cover:Math.min(1,Math.max(0,e.cover??.35)),hold:Math.min(2.5,Math.max(0,e.hold??.6)),away:Math.min(60,Math.max(0,e.away??1.5)),fade:Math.max(0,e.fade??.35)}}inputsKey(){let e=this.channels(),t=Object.values(e).flatMap(e=>xl(e)&&e.inputs?[e,...Object.values(e.inputs)]:[e]);return[...Cl(e,this.logOf(),this.on.items).map(e=>`${e.id}:${e.version}`),...t.filter(e=>yl(e)||bl(e)).map(e=>`r${e.relations.version}`)].join(`|`)}uniforms(e,t,n){let r=this.u,i=this.props,a=i.look??{},o=this.cols,s=i.rows??[],c=e=>this.names.indexOf(e),l=this.on.set?this.on.set.drive.smooth:0,u=(e.s.time.now.getTime()-l-o.epoch)/1e3,d=this.detailOf(e.s),f=this.step=d<.5?2:+(d<.75);r.fill(0);let p=this.room?.sets??[this],m=Math.min(30,p.filter(e=>e!==this&&(e.order>this.order||e.order===this.order&&e.added>this.added)).length);r.set(this.on.place(e,typeof i.lift==`number`?i.lift:0),0);let h=qt(i.size,`cards '${this.id}' size`),g=Ot(i.size);r.set([h[0],h[1],h[2],typeof g==`number`?g:120],4),r.set([6,18,15,s.length],8),r.set([u,f,this.pickNo,t],12),r.set([(i.opacity??1)*this.shown*this.on.shown,6,a.dots===!1?0:7,a.radius??5],16);let _=(e,t)=>L(e??t),v=this.timing();r.set(_(a.background,`#ffffffeb`),20),r.set([v.cover,v.hold,v.away,v.fade],24),r.set([this.stride,Math.max(t,this.slots),0,0],28),r.set(_(a.bar,`#2563eb`),32),r.set(_(a.ink,`#1f2937`),36),r.set(_(a.inkOnDark,`#ffffff`),40),r.set(_(a.badge,`#dc2626`),44),r.set([i.fill?c(`fill`):-1,this.fillDomain[0],this.fillDomain[1],+!!i.fill],48),r.set([i.badge?c(`badge`):-1,i.badge?c(`badgeOn`):-1,+!!i.badge,0],52),r.set([this.glyphM[0],this.glyphM[1],this.glyphM[2],1+m*2*262144],56);let y=this.room?.pics[0];r.set([y?.width??1,y?.height??1,4,+!!n],60),r.set([Math.floor(e.now)%63500,Math.min(.25,Math.max(0,e.dt/1e3)),0,+!!this.first],64);let b=i.icons;r.set([i.header?.icon&&b?1:0,i.header?.text===void 0?0:1,b?.cols??1,b?.rows??1],68),r.set([Math.ceil(Math.max(1,this.names.length)/4),this.names.length,m,Math.max(1,p.length)],72);let x=0;for(let e=0;e<6;e++){let t=s[e];r.set(t?t.text===void 0?[c(`r${e}`),t.of===void 0?-1:c(`r${e}of`),t.decimals??0,+!!t.bar]:[-2,x++,0,0]:[-1,-1,0,0],76+e*4);let n=this.barDomains[e]??[0,1];r.set([n[0],n[1],0,0],100+e*4);let i=this.labels[e];i&&(r.set(i.rect,124+e*4),r.set([i.w,i.h,0,0],148+e*4))}this.ubuf.write(r)}async decisions(){let e=this.on.items.length,t=new Float32Array(e*2);if(this.props.collide===!1)return t.fill(1),t;let n=this.states[this.cur],r=this.g;if(!n||!r)return null;let i=await r.readTexture(n.tex);for(let n=0;n<e;n++){let e=i[n*4];t[2*n]=+(e>=128),t[2*n+1]=e%128/127}return t}cardPlace(e){return this.on.lonLat(e,(this.attached?.world)?.time?.getTime())}get ids(){return this.on.ids}valuesOf(e,t){let n=this.cols;if(!n)return{};let r=n.at(this.on.ids[e]??e,t??(this.attached?.world)?.time?.getTime()??Date.now());return r?{...r.values}:{}}pickInfo(e){let t=this.on.items[e];if(t===void 0)return null;let n=(this.attached?.world)?.time?.getTime(),r=this.valuesOf(e,n);if(this.on.set){let t=this.on.set.pickInfo(e);return t?{...t,element:this.id,props:{...t.props,...r}}:null}let i=t;return{element:this.id,id:i.id??this.on.ids[e]??e,props:{...i.props??{},...r},d:t,index:e}}status(){let e=super.status();this.props.icons&&!this.props.icons.ready&&e.pending.push(`${this.id}: its pictures loading`),this.making&&e.pending.push(`${this.id}: its logs loading`),this.on.busy&&e.pending.push(`${this.id}: reading the ground under its places`),this.broken.length?e.failed.push(...this.broken.map(e=>({what:`${this.id} program`,error:e}))):this.wanted&&this.built<this.wanted&&e.pending.push(`${this.id}: its programs building`);let t=this.attached?.world;return this.props.collide!==!1&&t&&this.shown>0&&this.settling(t.now)&&e.pending.push(`${this.id}: making room`),e.ready=!e.pending.length,e}settling(e){let t=this.timing();return!this.decided||e-this.viewAt<(t.hold+t.away+t.fade)*1e3+100||e-this.touchedAt<t.fade*1e3+100}wantsFrames(e){if(this.on.busy)return 1/0;let t=this.attached?.world;return t?this.props.collide!==!1&&this.cols&&(this.settling(t.now)||t.now-this.madeAt<this.timing().fade*1e3+100)?1/0:(t.timeRunning===!0||e.time.rate)&&this.cols&&this.cols.samples>this.cols.size?15:0:0}readouts(){return{cards:()=>this.on.items.length,samples:()=>this.cols?.samples??0,channels:()=>this.names.length}}detach(){this.on.detach();for(let e of Object.values(this.progs))e?.destroy();this.progs={},this.progKey=``;for(let e of Object.values(this.tex))e?.destroy();this.tex={};for(let e of[...this.states,...this.carryFrom??[]])e.target.destroy(),e.tex.destroy();this.states=[],this.stateRows=0,this.carryFrom=null,this.stateIds=[],this.slotIds=[],this.stateN=this.slotN=0,this.gone.clear(),this.carry=!1,this.atlas=null,this.pics.clear(),this.atlasH=0,this.deriver=null;let e=this.room;if(e&&(e.sets=e.sets.filter(e=>e!==this),!e.sets.length)){for(let t of e.pics)t.destroy();e.pics=[],e.size=``}this.room=null,this.claimedAt=-2,this.ubuf?.destroy(),this.quads?.destroy(),this.dots?.destroy(),this.dummy?.destroy(),this.ubuf=this.quads=this.dots=null,this.dummy=null,this.cols=null,this.madeFor=-1,super.detach()}};function rv(e){return new nv(e)}function iv(e,t){let n=()=>{if(typeof t.for!=`string`)return t.for;if(!e)throw new K(`E-DATA-001`,`a legend for a layer id needs the world`,{fix:`legend(w, { for: 'crime' })`});let n=e.element(t.for).props[t.prop??`color`];if(!me(n))throw new K(`E-DATA-001`,`layer '${t.for}' has no scale in '${t.prop??`color`}'`,{where:t.for,fix:`give it color: scale.quantile(…) (or pass the scale itself)`});return n},r=()=>({...n().legend({format:t.format}),...t.title?{title:t.title}:{}}),i=null,a=-1,o=null,s=e?.renderer?.canvas,c=t.into??(s&&`parentElement`in s?s.parentElement:null),l=e=>e.replace(/[&<>"]/g,e=>({"&":`&amp;`,"<":`&lt;`,">":`&gt;`,'"':`&quot;`})[e]),u=()=>{if(!i)return;let e=r(),t=e.kind===`continuous`?`<div style="height:10px;border-radius:3px;background:linear-gradient(90deg,${(e.stops??[]).join(`,`)})"></div><div style="display:flex;justify-content:space-between;gap:8px;opacity:.85">${e.items.map(e=>`<span>${l(e.label)}</span>`).join(``)}</div>`:e.items.map(e=>`<div style="display:flex;align-items:center;gap:6px"><span style="width:12px;height:12px;border-radius:3px;background:${e.color??`#888`};flex:none"></span><span>${l(e.label)}</span></div>`).join(``)+(e.missing?`<div style="display:flex;align-items:center;gap:6px;opacity:.7"><span style="width:12px;height:12px;border-radius:3px;background:${e.missing};flex:none"></span><span>no data</span></div>`:``);i.innerHTML=`${e.title?`<div style="font-weight:600;margin-bottom:4px">${l(e.title)}</div>`:``}${t}`};if(c&&typeof document<`u`){if(i=document.createElement(`div`),t.className)i.className=t.className;else if(t.into)Object.assign(i.style,{font:`11px/1.4 system-ui, sans-serif`,color:`#e6ebf0`,pointerEvents:`none`});else{let[e,n]=(t.position??`bottom-left`).split(`-`);Object.assign(i.style,{position:`absolute`,[e]:`calc(10px + env(safe-area-inset-${e}, 0px))`,[n]:`10px`,zIndex:`10`,background:`rgba(12,16,24,.82)`,color:`#e6ebf0`,font:`11px/1.4 system-ui, sans-serif`,padding:`7px 9px`,borderRadius:`8px`,border:`1px solid rgba(255,255,255,.12)`,minWidth:`120px`,maxWidth:`220px`,pointerEvents:`none`}),getComputedStyle(c).position===`static`&&(c.style.position=`relative`)}i.className||=`aslnut-legend`,c.appendChild(i),u(),e&&(o=e.on(`frame`,()=>{try{let e=n().version;e!==a&&(a=e,u())}catch{}}))}return{el:i,describe:r,update:u,remove:()=>{o?.(),i?.remove(),i=null}}}const av=e=>({levels:[e],parts:[],anchors:[],triangles:[e.length/18],bounds:[[0,0,0],[1,1,1]]});function ov(e){return{walker:[-.8,-.4,0,.4,.8].map(t=>av(Ce(n=>{let r=(t,r)=>{let i=Math.sin(r)*.41,a=Math.cos(r)*.82;n.box(t,i,.85-a,.16,a,.18,0,e.pants)},i=(t,r)=>{let i=Math.sin(r)*.3,a=Math.cos(r)*.6;n.box(t,-i,1.42-a,.11,a,.12,0,e.shirt)};n.box(0,0,.85,.42,.6,.24,0,e.shirt),n.blob(0,1.62,0,.15,.15,.15,e.skin,1),r(-.1,t),r(.1,-t),i(-.28,-t*.8),i(.28,t*.8)}))),car:av(Ce(t=>{t.box(0,0,.32,1.8,.6,4.2,0,e.car),t.box(0,.2,.92,1.6,.55,2.1,0,e.cabin);for(let[e,n]of[[-.85,-1.35],[.85,-1.35],[-.85,1.35],[.85,1.35]])t.box(e,n,0,.26,.68,.68,0,`#111111`)})),drone:av(Ce(t=>{t.box(0,0,-.08,.5,.16,.5,0,e.drone);for(let[e,n]of[[-.42,-.42],[.42,-.42],[-.42,.42],[.42,.42]])t.box(e/2,n/2,-.02,.06,.04,.6,Math.atan2(n,e)+Math.PI/2,`#374151`),t.cylinder(e,n,.06,.08,.22,.22,12,`#9ca3af`)}))}}var sv=class{element=new Gu({id:`avatar`});mode=`map`;me;w;o;made=null;get shapes(){if(this.made)return this.made;let e=this.o,t=e.shapes??{},n=!t.walker||!t.car||!t.drone?ov({shirt:`#2f6fb0`,pants:`#2b2f3a`,skin:`#e0b48f`,car:`#c0392b`,cabin:`#2c3e50`,drone:`#e5e7eb`,...e.colors}):null;return this.made={walker:t.walker?[0,.25,.5,.75,1].map(t.walker):n.walker,car:t.car?.()??n.car,drone:t.drone?.()??n.drone}}stop=null;offs=[];ui=null;minDist0;constructor(e,t={}){if(this.w=e,this.o=t,this.minDist0=e.camera.minDist,this.view=t.camera??`follow`,this.me=Wu({id:`me`,kind:`person`,placement:`thing`,at:[e.camera.lon,e.camera.lat],shape:e=>this.view===`first-person`&&this.mode!==`drone`?null:this.mode===`car`?this.shapes.car:this.mode===`drone`?this.shapes.drone:this.shapes.walker[Math.round((Math.sin(e.gait)*Math.min(1,e.speed/2.8)*.5+.5)*4)],body:()=>this.mode===`walk`&&this.view!==`first-person`?this.bodyNow?.figure()??null:null}),this.setBody(t.body??null),typeof window<`u`){let e=e=>{this.mode!==`map`&&e.code===`Escape`&&this.setMode(`map`),e.code===`KeyF`&&t.vehicles&&this.enterOrExit(),e.code===`KeyV`&&this.mode!==`map`&&this.setCamera(this.view===`follow`?`first-person`:`follow`)};addEventListener(`keydown`,e),this.offs.push(()=>removeEventListener(`keydown`,e))}}view=`follow`;bodyNow=null;setBody(e){return this.bodyNow=e,e?.palette&&this.me.setPalette(e.palette),e?.decal&&(this.me.decal=[e.decal[0],e.decal[1],e.decal[2]]),this.me.scale=e?.scale??1,this.w.invalidate(),this}setCamera(e){return e===this.view||(this.view=e,this.mode!==`map`&&(this.stop?.(),this.drive(this.mode),this.w.invalidate())),this}drive(e){let t=this.view===`first-person`&&e!==`drone`;this.stop=Yu(this.w,this.element,this.me,e===`walk`?zu.walk():e===`car`?zu.ride():zu.fly(),t?{eye:e===`walk`?1.62:1.15}:{offset:e===`walk`?[0,2,-7]:e===`car`?[0,3.5,-12]:[0,8,-17],blocked:e===`drone`?void 0:this.o.obstacle})}riding=null;enterOrExit(){let e=this.o.vehicles;if(!e)return!1;let t=this.me,n=111320*Math.cos(t.at[1]*Math.PI/180);if(this.mode===`car`&&this.riding){let r=this.riding,i=t.heading*Math.PI/180,a=1.8;return e.onExit?.({id:r.id,at:[t.at[0],t.at[1]],heading:t.heading,made:r.made}),this.riding=null,this.made&&={...this.made,car:this.carMade0??this.made.car},t.at=[t.at[0]+Math.cos(i)*a/n,t.at[1]-Math.sin(i)*a/110574,NaN],this.setMode(`walk`),!0}if(this.mode!==`walk`)return!1;let r=null,i=3;for(let a of e.near()){let e=a.made.anchors.find(e=>e.kind===`entry`||e.name===`entry`||e.kind===`seat`||e.name===`driver`),o=a.heading*Math.PI/180,s=e?e.at[0]:0,c=e?e.at[2]:0,l=o+Math.PI,u=s*Math.cos(l)-c*Math.sin(l),d=s*Math.sin(l)+c*Math.cos(l),f=Math.hypot((a.at[0]-t.at[0])*n+u,(a.at[1]-t.at[1])*110574-d);f<i&&(i=f,r=a)}if(!r)return!1;e.onEnter?.(r.id);let a=this.shapes;return this.carMade0??=a.car,this.made={...a,car:r.made},this.riding={id:r.id,made:r.made},t.at=[r.at[0],r.at[1],NaN],t.heading=r.heading,this.element.look=r.heading*Math.PI/180,this.setMode(`car`),!0}carMade0=null;get kmh(){return this.mode===`map`?0:this.me.speed*3.6}setMode(e){if(e===this.mode)return this;let t=this.w,n=t.camera,r=this.me;if(this.stop?.(),this.stop=null,this.mode===`map`){r.at=[n.lon,n.lat,NaN],r.heading=n.view.bearing;let t=e===`drone`?null:this.o.start?.([n.lon,n.lat]);t&&(r.at=[t.at[0],t.at[1],NaN],r.heading=t.heading/G),r.velocity=[0,0,0],r.speed=0}return e===`drone`&&this.mode!==`drone`&&(r.altitude=25),this.mode=e,e===`map`?(this.element.remove(`me`),t.camera.minDist=this.minDist0,t.fly({lon:r.at[0],lat:r.at[1],zoom:17.2,tilt:58,bearing:this.element.look/G},{seconds:1})):(this.element.get(`me`)||this.element.add(r),r.locomotion=e===`walk`?ju({speed:5/3.6,run:5,blocked:this.o.obstacle}):e===`car`?ju({wheels:{},blocked:this.o.obstacle}):Nu({ceiling:400}),t.camera.minDist=1.5,this.drive(e)),this.touchUI(),this.o.onMode?.(e),t.invalidate(),this}touchUI(){let e=this.o.touch??(typeof matchMedia<`u`&&matchMedia(`(pointer: coarse)`).matches);if(this.mode===`map`||!e||typeof document>`u`){this.ui?.remove(),this.ui=null;return}if(this.ui)return;let t=document.createElement(`div`);this.ui=t;let n=this.element.input;Object.assign(t.style,{position:`fixed`,left:`18px`,bottom:`calc(84px + env(safe-area-inset-bottom))`,width:`120px`,height:`120px`,borderRadius:`60px`,background:`rgba(255,255,255,.12)`,border:`1px solid rgba(255,255,255,.3)`,zIndex:`20`,touchAction:`none`});let r=document.createElement(`div`);Object.assign(r.style,{position:`absolute`,left:`40px`,top:`40px`,width:`40px`,height:`40px`,borderRadius:`20px`,background:`rgba(255,255,255,.55)`}),t.appendChild(r);let i=-1,a=e=>{let i=t.getBoundingClientRect(),a=i.left+60,o=i.top+60,s=(e.clientX-a)/50,c=(e.clientY-o)/50,l=Math.hypot(s,c);l>1&&(s/=l,c/=l),n.stick=[s,c],this.w.invalidate(),r.style.left=`${40+s*40}px`,r.style.top=`${40+c*40}px`};t.addEventListener(`pointerdown`,e=>{i=e.pointerId,t.setPointerCapture(e.pointerId),a(e),e.stopPropagation()}),t.addEventListener(`pointermove`,e=>{e.pointerId===i&&a(e)});let o=e=>{e.pointerId===i&&(i=-1,n.stick=[0,0],r.style.left=`40px`,r.style.top=`40px`)};t.addEventListener(`pointerup`,o),t.addEventListener(`pointercancel`,o);let s=(e,r,i)=>{let a=document.createElement(`button`);a.textContent=e,Object.assign(a.style,{position:`fixed`,right:`18px`,bottom:`calc(${r}px + env(safe-area-inset-bottom))`,width:`56px`,height:`56px`,borderRadius:`28px`,background:`rgba(255,255,255,.16)`,color:`#fff`,border:`1px solid rgba(255,255,255,.35)`,fontSize:`20px`,zIndex:`20`,touchAction:`none`}),a.addEventListener(`pointerdown`,e=>{n.up=i,this.w.invalidate(),e.stopPropagation()}),a.addEventListener(`pointerup`,()=>{n.up=0}),a.addEventListener(`pointercancel`,()=>{n.up=0}),t.appendChild(a)};if(s(`▲`,150,1),s(`▼`,84,-1),this.o.vehicles){let e=document.createElement(`button`);e.textContent=`⇄`,e.title=`get in / out`,Object.assign(e.style,{position:`fixed`,right:`18px`,bottom:`calc(216px + env(safe-area-inset-bottom))`,width:`56px`,height:`56px`,borderRadius:`28px`,background:`rgba(255,255,255,.16)`,color:`#fff`,border:`1px solid rgba(255,255,255,.35)`,fontSize:`20px`,zIndex:`20`,touchAction:`none`}),e.addEventListener(`pointerdown`,e=>{e.stopPropagation(),this.enterOrExit()}),t.appendChild(e)}{let e=document.createElement(`button`);e.textContent=`◉`,e.title=`camera: behind / eyes`,Object.assign(e.style,{position:`fixed`,right:`84px`,bottom:`calc(84px + env(safe-area-inset-bottom))`,width:`56px`,height:`56px`,borderRadius:`28px`,background:`rgba(255,255,255,.16)`,color:`#fff`,border:`1px solid rgba(255,255,255,.35)`,fontSize:`20px`,zIndex:`20`,touchAction:`none`}),e.addEventListener(`pointerdown`,e=>{e.stopPropagation(),this.setCamera(this.view===`follow`?`first-person`:`follow`)}),t.appendChild(e)}document.body.appendChild(t)}dispose(){this.stop?.();for(let e of this.offs)e();this.offs=[],this.ui?.remove(),this.ui=null,this.w.camera.minDist=this.minDist0}};const cv=(e,t={})=>new sv(e,t);function lv(e,t={}){let n=t.radius??.3,r=t.step??.35,i;if(!t.solids||t.solids===`compiled`)i=(t,n)=>{let r=0;for(let i of e.elementsOfKind(`shape`)){let e=i.blockerAt;e&&(r=Math.max(r,e.call(i,t,n)))}return r};else{let e=t.solids,n=-1,r=/* @__PURE__ */ new Map,a=0,o=()=>{let i=e instanceof Te?e.features():e;if(e instanceof Te){if(e.version===n)return;n=e.version}r=/* @__PURE__ */ new Map,a=0;for(let e of i){let n=e.geometry;if(!n||n.type!==`Polygon`&&n.type!==`MultiPolygon`)continue;let i=t.height?.(e)??Number(e.props?.height??6);for(let e of n.type===`Polygon`?[n.coordinates]:n.coordinates){let t=e[0],n=1/0,o=-1/0,s=1/0,c=-1/0;for(let[e,r]of t)n=Math.min(n,e),o=Math.max(o,e),s=Math.min(s,r),c=Math.max(c,r);a||=111320*Math.cos((s+c)/2*G);for(let e=Math.floor(n*a/20);e<=Math.floor(o*a/20);e++)for(let n=Math.floor(s*110574/20);n<=Math.floor(c*110574/20);n++){let a=`${e}_${n}`,o=r.get(a);o||r.set(a,o=[]),o.push({ring:t,h:i})}}}};o(),i=(e,t)=>{if(o(),!a)return 0;let n=0;for(let i of r.get(`${Math.floor(e*a/20)}_${Math.floor(t*110574/20)}`)??[])i.h>n&&uv(i.ring,e,t)&&(n=i.h);return n}}return(e,t)=>{let a=111320*Math.cos(t*G),o=i(e,t);if(n>0)for(let r=0;r<8;r++){let s=r/8*Math.PI*2;o=Math.max(o,i(e+Math.cos(s)*n/a,t+Math.sin(s)*n/110574))}return o<r?0:o}}const uv=(e,t,n)=>{let r=!1;for(let i=0,a=e.length-1;i<e.length;a=i++){let[o,s]=e[i],[c,l]=e[a];s>n!=l>n&&t<(c-o)*(n-s)/(l-s)+o&&(r=!r)}return r},{createContext:dv,createElement:fv,useCallback:pv,useContext:mv,useEffect:hv,useRef:gv,useState:_v,useSyncExternalStore:vv}=globalThis.__zebReact,yv=dv(null);function bv(e){let t=gv(null),n=gv(null),[r,i]=_v(null),a=!!e.headless;return hv(()=>{let r=e.options??{},o=a?null:document.createElement(`canvas`);o&&(o.style.cssText=`display:block;width:100%;height:100%;touch-action:none`,t.current?.appendChild(o));let s=!0,c=null;return yo(o,r).then(e=>{s?(c=e,n.current=Eo(e,{options:r}),i(e)):e.dispose()},e=>{s&&(r.onError??console.error)(e)}),()=>{s=!1,n.current=null,c?.dispose(),c=null,i(null),o?.remove()}},[a]),hv(()=>{r&&n.current?.elements(e.elements??[])},[r,e.elements]),hv(()=>{r&&n.current?.options(e.options??{},e.viewChange)},[r,e.options,e.viewChange]),hv(()=>{r&&e.onReady?.(r)},[r]),fv(yv.Provider,{value:r},fv(`div`,{className:e.className,style:{position:`relative`,...e.style}},fv(`div`,{ref:t,style:{position:`absolute`,inset:0}}),e.children))}function xv(){return mv(yv)}function Sv(e,t){let n=mv(yv),r=t===void 0?n:t,i=pv(e=>{if(!r)return()=>{};let t=r.on(`elements`,e);return()=>{t()}},[r]);return vv(i,()=>r&&!r.isDisposed?r.list.find(t=>t.id===e)??null:null,()=>null)}function Cv(e,t,n){let r=mv(yv),i=n===void 0?r:n,a=gv(t);hv(()=>{a.current=t}),hv(()=>{if(!i||i.isDisposed)return;let t=i.on(e,(...e)=>a.current(...e));return()=>{t()}},[i,e])}const wv=new URL(`./tiles.worker.mjs`,import.meta.url).href;export{Rc as Arc,Ho as Area,K as AslnutError,sv as Avatars,xg as BAKED_LOOK,Qe as BEHIND,Gm as BLOCK,Dg as BakedRoads,sn as Builder,Gg as Buildings,ag as CITY,lg as CITY_KINDS,Rp as CLEAR,Cn as CSV,V as CUSTOM_SCOPE,wr as Camera,nv as Cards,dl as Custom,Tt as CustomMaterial,ml as CustomMesh,Er as DAMPING,Dd as DEFAULT_HEAT_OPTIONS,Yt as DEFAULT_LADDER,au as DQ_FLOATS,On as DRAPE_DEFAULTS,mc as DataDrive,rn as EASES,yn as EASE_PRESETS,nm as EarthSky,D as Element,rc as Entities,Uu as Entity,Gu as EntityElement,B as EntityKind,bc as EntitySet,es as Extrusion,Ut as FeatureMark,br as Flight,Fr as Follower,At as GATES,zn as GROUND_WITHIN,Ya as Gates,en as GeoJSON,Xa as Governor,Ln as GraphicsUnavailable,Kl as Groups,Wl as GroupsSource,Tr as HALF_LIFE_K,hf as HEAT_DIFF_RAMP,pf as HEAT_METRICS,Od as HEAT_OPTION_INFO,mf as HEAT_RAMP,un as HOOK_POINTS,A_ as HOT_HOUR,Wm as HOUSE,vf as Heat,Lt as INSTANCE_FLOATS,Oc as Icon,Ec as IconAtlas,Qc as ImageRaster,wt as Join,ne as Joined,ua as KIND_ORDER,$e as LAMP_CODES,ja as LEAF_FILL,kn as LINE_CODE,$m as LOT,rs as Layer,Et as Levels,Lo as Line,Li as Loader,Ai as LoaderMiss,Gs as Log,gd as MATERIALS,Jt as MATERIALS_LOOK,Bn as MATERIAL_CODES,ie as MESH_FLOATS,he as MODES,ce as MeshBuilder,et as NO_OBSERVATION,$u as Network,xr as PATHS,Om as PLAN,gn as PMTiles,Me as PMTilesReader,S as PMTilesWriter,hh as POLES,dt as PRESET_NOTES,wc as Placement,Jo as Points,Ft as Query,oe as RAMPS,xh as ROAD_CLASS,fe as ROOF_FORMS,E_ as Rain,Zc as Raster,uf as RasterField,nl as RegionTree,Zs as Relations,Za as RemoteLog,la as Renderer,Rg as Roads,at as Rows,t_ as SEA_LOOK,ge as SHAPE_FLOATS,s_ as SHORE,mu as SKIN_FLOATS,hu as SKIN_INSTANCE_FLOATS,gu as SKIN_SLOTS,Uh as SPECIES,fa as STATE_SCHEMA,Sh as STREET,Oh as STREETS_LOOK,eo as Scenarios,b as Scope,h_ as SeaSurface,ye as ShaderError,Gn as Shape,Ga as ShapePackage,Ja as ShapeShelf,ti as Shot,ou as Skeleton,Tu as SkinDraw,Df as Sky,Zt as Solids,Te as Source,L_ as Stats,an as Stream,cm as StreetDetail,dg as Streets,up as Surface,ao as THEMES,or as TILT_LIMIT,ar as TILT_MAX,Kh as TREES,_n as Text,Oe as Tiled,Vc as Trail,vm as VIC_STREETS,rg as Vegetation,_d as WALL,ch as WATER,zp as WEATHERS,ft as WFS,ph as Water,_h as Wires,bv as World,Ve as anchorOf,zc as arc,Uo as area,I_ as areaKm2,Ge as asKind,Le as asksLevel,cv as avatars,xu as bakeClip,wg as bakedMesh,Og as bakedRoads,Sg as bakedRoadsWorker,O as bboxOf,nh as bins,fd as blackbody,La as branching,Jg as buildings,rv as cards,Np as catalogues,H as centreOf,Ll as channels,x_ as charger,S_ as chargers,Or as checkDamping,pr as checkView,u as choice,P_ as cityHeat,M_ as cityRasters,cg as cityTile,jg as clampTerrain,u_ as coastField,c_ as coastSegments,st as codeOf,lv as collision,ze as color,ue as colours,rp as composeHeights,Cf as constants,dn as contains,Yu as control,zu as controllers,md as convection,Gt as cubicBezier,fl as custom,bn as data,hc as dataDrive,Qf as decodeHeight,m as decodeMVT,rr as distForZoom,y as drapeOf,Bu as drive,eh as driveway,s as earcut,hp as earth,mp as earthLite,En as ease,ht as easing,Pn as encodeMVT,Cc as entities,Wu as entity,Ku as entityElement,Dn as epsgOf,gt as exceptionOf,ts as extrusion,th as fence,ff as field,Cu as figure,N as flag,Ju as followEntity,Xm as footprint,Ze as footprintOf,Fe as form,je as geo,Ip as gmst,g as grammar,Nc as greatCircle,tp as gridSampler,Nd as groundSun,Jl as groups,Bl as groupsJob,yf as heat,jd as heatModel,N_ as heatRaster,E as hex,Qs as history,Zm as house,Am as housePlan,kc as icon,Dc as icons,w as icosphere,fp as imagery,tu as importShape,Qt as impostor,An as inLevels,He as int,qm as isHouse,mn as isImpostor,bo as isMiss,me as isScale,mo as isTheme,Ht as jenksBreaks,Fh as junctionMesh,of as kernel,Sr as keys,xe as kind,n as kinds,Ua as kitbash,Vn as ladderOf,is as layer,G_ as layers,vr as leg,yr as legCost,iv as legend,Ro as line,z as linesOf,Gi as liveGraphics,Ui as loader,l_ as localFrame,qn as localTime,Pu as locomotion,Lr as lookAt,ir as makeState,In as material,De as meshCode,wp as moonAt,Sn as motionFrames,Vu as motions,Yr as move,$r as moves,td as network,qe as normalize,k as num,oa as openGraphics,O_ as openMeteo,F as packNormal,v as parseCSV,Mp as parseStars,f as partOf,Op as phaseName,ur as pitchFloor,Tc as placement,vp as plane,gp as planeAt,_p as planeXY,Yo as points,We as pointsOf,gh as poles,Xe as polygonsOf,hr as poseOf,Ha as preset,St as presetCurve,o as prettyBreaks,Xu as preview,ym as profileFor,ca as profileOf,Eh as propertyLine,lu as qaxis,cu as qmul,D_ as rain,ve as ramp,tl as raster,Fm as reachInto,ln as readWFSCapabilities,Xt as readWFSDescribe,jt as readWMSCapabilities,Dt as readXML,Wa as recipes,Pm as rectInside,il as regions,Xs as replay,C as reveal,W as rgbOf,L as rgba,Wr as rigFrom,Ur as rigPose,wh as roadSection,zg as roads,Bt as roofFaces,_t as roofMeasure,qg as roofMesh,Ug as roofOf,i as roofRise,du as rotateBone,_u as rowTexels,sf as ruleOf,rf as rules,ld as saturationVapour,Rt as scale,to as scenarioApi,g_ as sea,Nh as sectionMesh,gl as serve,it as shadowBox,U as shape,Nn as shapeCode,Ce as shapeOf,Ka as shapePackage,nu as shapes,Eo as shell,j as shown,Dh as sidewalks,on as sizePxAt,qt as sizeUnits,Ot as sizeValue,su as skeleton,yu as skinMesh,fu as skinPoint,im as sky,Tp as skyDir,dd as skyLongwave,em as skyNow,Lp as skyTurn,Pd as skyViewFactor,Ys as smallest,bu as smoothNormals,nt as solids,bd as solveSteady,da as sortElements,Gh as speciesOf,Ae as splitParts,cn as spring,Ar as springStep,Fp as starData,Oa as states,R_ as stats,Sd as stepSurface,ku as still,uh as stream,Th as streetBands,lm as streetDetail,fg as streets,ug as streetsWorker,Cp as sunAt,hn as sunFrames,Md as sunTest,Ep as sunTimes,aa as supported,xd as surfaceCapacity,pp as terrain,lt as text,fn as textParam,po as theme,np as tileHeights,wv as tilesWorkerUrl,lr as tiltMaxAt,Ir as track,Hc as trail,xn as traits,e as translateModule,yt as translateStatements,Zh as treeCrown,Xh as treeInstances,Yh as treeShapes,kt as triangulate,e_ as typology,Sv as useElement,xv as useWorld,Cv as useWorldEvent,wd as utci,Ed as utciStress,Pe as valueOf,ud as vapourPressure,c as vary,ig as vegetation,gr as viewOf,Bp as visibilityOf,mh as water,lh as waterArea,a_ as waveSet,zr as whereOf,dh as widthOf,pd as windProfile,Kg as windowRows,vh as wires,hl as worker,vl as workers,yo as world,Su as writeRow,Qn as zoneLookups,Jn as zonedTime,nr as zoomForDist};