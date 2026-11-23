import{p as T,s as v,a as U,d as R,c as O,b as j,e as x,o as P,f as z,l as V,g as I,h as N}from"./voice-catalog-BPNuR_vZ.js";import{_ as A}from"./play.js";function $(n){let e;try{e=JSON.parse(n)}catch{throw new Error("The voice config is not JSON. Choose the .onnx.json file that belongs to the Piper voice.")}const t=e,r=Number(t.audio?.sample_rate);if(!Number.isInteger(r)||r<8e3||r>48e3)throw new Error("The voice config has no usable audio.sample_rate.");if(t.phoneme_type!==void 0&&t.phoneme_type!=="espeak")throw new Error(`Only espeak phoneme voices are supported, this one uses ${String(t.phoneme_type)}.`);const s=t.phoneme_id_map;if(!s||typeof s!="object"||Array.isArray(s))throw new Error("The voice config has no phoneme_id_map.");const i={};for(const[c,h]of Object.entries(s))Array.isArray(h)&&h.length>0&&h.every(d=>Number.isInteger(d))&&(i[c]=h);for(const c of["^","$","_"])if(!i[c])throw new Error(`The voice config's phoneme_id_map lacks "${c}".`);const o=(c,h)=>typeof c=="number"&&Number.isFinite(c)?c:h,a=typeof t.espeak?.voice=="string"&&t.espeak.voice?t.espeak.voice:"en-us",l=typeof t.language?.name_english=="string"&&t.language.name_english?t.language.name_english:a;return{sampleRate:r,espeakVoice:a,language:l,phonemeIdMap:i,noiseScale:o(t.inference?.noise_scale,.667),lengthScale:o(t.inference?.length_scale,1),noiseW:o(t.inference?.noise_w,.8),numSpeakers:Math.max(1,Math.round(o(t.num_speakers,1)))}}function F(n){return Object.entries(n.phonemeIdMap).filter(([e])=>[...e].length===1&&!/[\r\n]/.test(e)).map(([e,t])=>`${e} ${t[0]}
`).join("")}function L(n){return{model_type:"vits",comment:"piper",language:n.language,voice:n.espeakVoice,has_espeak:"1",n_speakers:String(n.numSpeakers),sample_rate:String(n.sampleRate)}}function D(n){const e=t=>String(n.phonemeIdMap[t][0]);return{frontend:"characters",add_blank:"1",use_eos_bos:"1",bos_id:e("^"),eos_id:e("$"),blank_id:e("_"),pad_id:e("_")}}function C(n){return Object.keys(n.phonemeIdMap).filter(e=>[...e].length===1).join("")}function W(n){const e=new Set;let t=0;for(;t<n.length;){const[r,s]=f(n,t),i=Math.floor(r/8),o=r%8;if(o===0){t=f(n,s)[1];continue}if(o===1){t=s+8;continue}if(o===5){t=s+4;continue}if(o!==2)throw new Error("The voice model is not an ONNX file.");const[a,l]=f(n,s),c=l+a;if(c>n.length)throw new Error("The voice model is not an ONNX file (truncated).");if(i===14){const h=J(n.subarray(l,c));h[1]!==void 0&&e.add(h[1])}t=c}return e}function q(n,e,t="espeak"){const r=W(n),s=new TextEncoder,i=[],o=L(e);if(t==="potoru"){const a=D(e),l=Object.keys(a).filter(c=>r.has(c));if(l.length)throw new Error(`This voice model already sets ${l.join(", ")}; the Potoru phonemizer needs a plain Piper export. Use the espeak phonemizer for it.`);Object.assign(o,a)}for(const[a,l]of Object.entries(o)){if(r.has(a))continue;const c=y([w(1,s.encode(a)),w(2,s.encode(l))]);i.push(w(14,c))}return i.length?y([n,...i]):n}function f(n,e){let t=0,r=1;for(let s=e;s<n.length&&s<e+10;s+=1){const i=n[s];if(t+=(i&127)*r,i<128)return[t,s+1];r*=128}throw new Error("The voice model is not an ONNX file.")}function J(n){const e={};let t=0;for(;t<n.length;){const[r,s]=f(n,t);if(r%8!==2)return e;const[i,o]=f(n,s);e[Math.floor(r/8)]=new TextDecoder().decode(n.subarray(o,o+i)),t=o+i}return e}function S(n){const e=[];let t=n;for(;t>=128;)e.push(t%128|128),t=Math.floor(t/128);return e.push(t),e}function w(n,e){return y([Uint8Array.from([...S(n*8+2),...S(e.length)]),e])}function y(n){const e=new Uint8Array(n.reduce((r,s)=>r+s.length,0));let t=0;for(const r of n)e.set(r,t),t+=r.length;return e}function B(n,e){const t=new Uint8Array(44+n.length*2),r=new DataView(t.buffer),s=(i,o)=>{for(let a=0;a<o.length;a+=1)r.setUint8(i+a,o.charCodeAt(a))};s(0,"RIFF"),r.setUint32(4,36+n.length*2,!0),s(8,"WAVE"),s(12,"fmt "),r.setUint32(16,16,!0),r.setUint16(20,1,!0),r.setUint16(22,1,!0),r.setUint32(24,e,!0),r.setUint32(28,e*2,!0),r.setUint16(32,2,!0),r.setUint16(34,16,!0),s(36,"data"),r.setUint32(40,n.length*2,!0);for(let i=0;i<n.length;i+=1){const o=Math.max(-1,Math.min(1,n[i]));r.setInt16(44+i*2,Math.round(o*32767),!0)}return t}const H="Run `node scripts/setup-tts-wasm.mjs` with POTORU_SHERPA_TTS_DIR set (see authoring/fixtures/DEMO-SLIDE-STORY-REPORT.md).";function G(n){return R(n.espeakVoice)}const K=()=>A(()=>import("./g2p-wasm-CWpByVDb.js"),[],import.meta.url).then(n=>n.loadG2p());function X(n,e=b,t=K){const r=$(n.config),s=n.phonemizer??G(r);if(s==="potoru"&&!T(r.espeakVoice))throw new Error(`The Potoru phonemizer supports voices trained on espeak "en" or "id" only in this version (this voice uses espeak voice "${r.espeakVoice}"). Choose espeak.`);const i=C(r);let o;const a=()=>{const l=e();if(!o||o.engine!==l||o.generation!==l.generation){const c=new Uint8Array(n.model),h=q(c,r,s),d=h===c?c.slice():h,g={noiseScale:r.noiseScale,noiseScaleW:r.noiseW,lengthScale:r.lengthScale},u={generation:l.generation,engine:l,voice:l.load(d,F(r),g).then(p=>p.voice)};u.voice.catch(()=>{o===u&&(o=void 0)}),o=u}return o};return{provider:"sherpa",config:r,phonemizer:s,async synthesize(l,c={}){if(!/[\p{L}\p{N}]/u.test(v(l)))throw new Error("Nothing to say: the line has no words.");let h=v(l).replace(/\s+/g," ").trim(),d;if(s==="potoru"&&(d=U(await t(),l.replace(/\s+/g," ").trim(),c.lang,{symbols:i,espeakVoice:r.espeakVoice}),h=d.sentences.join(""),!h.trim()))throw new Error("Nothing to say: the Potoru G2P found no words it can read in this line.");const g=a(),u=await g.voice;if(c.signal?.aborted)throw new DOMException("Speech cancelled","AbortError");const{samples:p,sampleRate:m}=await g.engine.speak(u,h,0);if(!p.length)throw new Error("sherpa-onnx returned no audio for this line.");const M=B(p,m);return{blob:new Blob([M.buffer],{type:"audio/wav"}),durationSeconds:p.length/m,sampleRate:m,...d?{phonemes:d}:{}}},dispose(){const l=o;o=void 0,l&&l.voice.then(c=>l.engine.free(c),()=>{})}}}const Y=String.raw`
"use strict";
let boot;
let lastError = "";
const voices = new Map();
let nextVoice = 1;
function start(base) {
  boot ??= (async () => {
    const response = await fetch(base + "manifest.json").catch(() => undefined);
    const manifest = response && response.ok && /json/.test(response.headers.get("content-type") || "") ? await response.json().catch(() => undefined) : undefined;
    if (!manifest || manifest.engine !== "sherpa-onnx-tts-wasm") throw new Error("The sherpa-onnx TTS files are not installed at " + base + ".");
    await new Promise((resolve, reject) => {
      self.Module = {
        locateFile: (path) => base + path,
        print: () => {},
        printErr: (text) => { lastError = String(text); },
        setStatus: () => {},
        onRuntimeInitialized: resolve,
        onAbort: (what) => reject(new Error("sherpa-onnx stopped: " + what))
      };
      try { importScripts(base + "sherpa-onnx-tts.js", base + "sherpa-onnx-wasm-main-tts.js"); }
      catch (error) { reject(new Error("The sherpa-onnx TTS scripts could not be loaded from " + base + ".")); }
    });
    self.Module.FS_createPath("/", "voices", true, true);
    return self.Module;
  })();
  return boot;
}
function quiet(action) {
  const log = console.log;
  console.log = () => {};
  try { return action(); } finally { console.log = log; }
}
async function handle(message) {
  const Module = await start(message.base);
  if (message.type === "load") {
    const id = nextVoice++;
    const dir = "/voices/" + id;
    Module.FS_createPath("/voices", String(id), true, true);
    Module.FS_createDataFile(dir, "model.onnx", new Uint8Array(message.model), true, false, true);
    Module.FS_createDataFile(dir, "tokens.txt", new TextEncoder().encode(message.tokens), true, false, true);
    lastError = "";
    const tts = quiet(() => createOfflineTts(Module, {
      offlineTtsModelConfig: {
        offlineTtsVitsModelConfig: { model: dir + "/model.onnx", lexicon: "", tokens: dir + "/tokens.txt", dataDir: "/espeak-ng-data", dictDir: "", noiseScale: message.params.noiseScale, noiseScaleW: message.params.noiseScaleW, lengthScale: message.params.lengthScale },
        numThreads: 1, debug: 0, provider: "cpu"
      },
      ruleFsts: "", ruleFars: "", maxNumSentences: 1
    }));
    // The model is read into the engine; drop the file copy to free memory.
    try { Module.FS_unlink(dir + "/model.onnx"); } catch {}
    if (!tts.handle) throw new Error("sherpa-onnx could not load this voice" + (lastError ? ": " + lastError : "."));
    voices.set(id, tts);
    return { voice: id, sampleRate: tts.sampleRate, numSpeakers: tts.numSpeakers };
  }
  if (message.type === "speak") {
    const tts = voices.get(message.voice);
    if (!tts) throw new Error("This voice is not loaded.");
    const audio = tts.generate({ text: message.text, sid: message.speakerId, speed: 1.0 });
    return { samples: audio.samples, sampleRate: audio.sampleRate };
  }
  if (message.type === "free") {
    const tts = voices.get(message.voice);
    voices.delete(message.voice);
    if (tts) tts.free();
    return {};
  }
  throw new Error("Unknown request " + message.type);
}
let queue = Promise.resolve();
self.onmessage = (event) => {
  const message = event.data;
  queue = queue.then(() => handle(message)).then((result) => {
    self.postMessage({ id: message.id, ok: true, result }, result.samples ? [result.samples.buffer] : []);
  }, (error) => {
    const text = error && error.message ? error.message : String(error);
    // A failed start or an aborted runtime leaves nothing usable: say so, so the host starts afresh.
    self.postMessage({ id: message.id, ok: false, error: text, fatal: !boot || /stopped|not installed|could not be loaded|abort|exit/i.test(text) });
  });
};
`;class Q{constructor(e){this.base=e}base;generation=0;worker;workerUrl;nextId=1;pending=new Map;load(e,t,r){const s=e.byteOffset===0&&e.byteLength===e.buffer.byteLength?e.buffer:e.slice().buffer;return this.call({type:"load",model:s,tokens:t,params:r},[s])}speak(e,t,r){return this.call({type:"speak",voice:e,text:t,speakerId:r})}free(e){this.worker&&this.call({type:"free",voice:e}).catch(()=>{})}call(e,t=[]){const r=this.ensureWorker(),s=this.nextId++;return new Promise((i,o)=>{this.pending.set(s,{resolve:i,reject:o}),r.postMessage({...e,id:s,base:this.base},t)})}ensureWorker(){if(this.worker)return this.worker;this.workerUrl=URL.createObjectURL(new Blob([Y],{type:"text/javascript"}));const e=new Worker(this.workerUrl);return e.onmessage=t=>{const{id:r,ok:s,result:i,error:o,fatal:a}=t.data,l=this.pending.get(r);if(this.pending.delete(r),s)l?.resolve(i);else{const c=/not installed|could not be loaded/.test(o??"");l?.reject(new Error(c?`${o} ${H}`:o??"sherpa-onnx failed.")),a&&this.restart(new Error(o??"sherpa-onnx stopped."))}},e.onerror=t=>{t.preventDefault(),this.restart(new Error(`sherpa-onnx worker failed${t.message?`: ${t.message}`:"."}`))},this.worker=e,e}restart(e){this.worker?.terminate(),this.worker=void 0,this.workerUrl&&URL.revokeObjectURL(this.workerUrl),this.workerUrl=void 0,this.generation+=1;for(const t of this.pending.values())t.reject(e);this.pending.clear()}}const k=new Map;function b(n){const e=new URL(n??"tts/",document.baseURI).href.replace(/\/?$/,"/");let t=k.get(e);return t||(t=new Q(e),k.set(e,t)),t}function Z(n){if(n.provider==="elevenlabs")return JSON.stringify(["elevenlabs",n.voiceId.trim(),n.modelId.trim()]);if(n.provider==="openai")return JSON.stringify(["openai",n.endpoint.trim().replace(/\/+$/,""),n.model.trim(),n.voice.trim()])}async function ee(n,e,t){const r=new TextEncoder().encode(JSON.stringify([n,t??"",e])),s=await crypto.subtle.digest("SHA-256",r);return[...new Uint8Array(s)].map(i=>i.toString(16).padStart(2,"0")).join("")}function te(n,e,t){return{provider:n.provider,async synthesize(r,s={}){let i;try{i=await ee(e,r,s.lang);const a=await t.getLine(i);if(a)return{blob:a.blob,...a.durationSeconds!==void 0?{durationSeconds:a.durationSeconds}:{}}}catch{}const o=await n.synthesize(r,s);return i&&t.putLine(i,o.blob,o.durationSeconds).catch(()=>{}),o}}}function re(n,e={}){switch(n.provider){case"sherpa":{const t=n.catalog?.engine;return X(n,t?()=>b(t):b)}case"browser":return x(n);case"elevenlabs":case"openai":{const t=n.provider==="elevenlabs"?O(n):j(n),r=Z(n);return e.cache&&r?te(t,r,e.cache):t}}}function ne(n){n.dispose?.()}class se{constructor(e=z()){this.store=e,P(()=>{this.reload()})}store;settings=[];synthesizers=new Map;problems=new Map;browser;listeners=new Set;catalogs=new Map;loading;generation=0;ready(){return this.loading??=this.reload(),this.loading}async reload(){const e=this.generation+=1;let t=[];try{t=await this.store.list({localDefaults:!1})}catch{t=[]}if(e===this.generation){for(const r of this.synthesizers.values())ne(r);this.settings=t,this.synthesizers=new Map,this.problems=new Map;for(const r of t)try{this.synthesizers.set(r.alias,re(r,{cache:this.store}))}catch(s){this.problems.set(r.alias,s instanceof Error?s.message:String(s))}for(const r of[...this.listeners])r()}}subscribe(e){return this.listeners.add(e),()=>{this.listeners.delete(e)}}get list(){return this.settings}setting(e){return this.settings.find(t=>t.alias===e)}problem(e){return this.problems.get(e)}get browserVoice(){if(this.browser===void 0)try{this.browser=x()}catch{this.browser=null}return this.browser??void 0}resolver(e){return{resolve:t=>this.synthesizers.get(t)??(e==="browser"?this.browserVoice:void 0)}}async assign(e){await this.store.put(e),await this.reload()}async unassign(e){await this.store.remove(e),await this.reload()}catalog(e){const t=new URL(e,document.baseURI).href;let r=this.catalogs.get(t);return r||(r=V(t),r.catch(()=>this.catalogs.delete(t)),this.catalogs.set(t,r)),r}installed(){return this.store.installed().catch(()=>[])}async install(e,t,r={}){const s=await I(t,{...r.onProgress?{onProgress:r.onProgress}:{},...r.signal?{signal:r.signal}:{}}),i=oe(e,t,s.sha256);await this.store.putModel(i,s.model,s.config),N(),r.alias?await this.assign(_(r.alias,i,s.model,s.config,t)):await this.reload()}async useInstalled(e,t){const r=await this.store.getModel(t.key);if(!r)throw new Error(`${t.ref.name} is no longer downloaded.`);await this.assign(_(e,r.ref,r.model,r.config))}async uninstall(e){await this.store.removeModel(e),await this.reload()}}function oe(n,e,t){return{id:e.id,url:n.url,name:e.name,sha256:t,...e.version?{version:e.version}:{},...n.engine?{engine:n.engine}:{}}}function _(n,e,t,r,s){const i=(o,a)=>{try{return o?decodeURIComponent(new URL(o).pathname.split("/").pop()||a):a}catch{return a}};return{alias:n,provider:"sherpa",modelName:i(s?.model,`${e.id}.onnx`),configName:i(s?.config,`${e.id}.onnx.json`),model:t,config:r,catalog:e}}let E;function ce(){return E??=new se,E}export{se as PlayerVoices,ce as playerVoices};
