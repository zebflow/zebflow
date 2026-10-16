import{a as P,b as V,c as b,d as E,h as z,l as M,m,o as U,p as _,r as R,s as A,t as T}from"./chunk-BNXQLZB7.mjs";import"./chunk-O26L46OV.mjs";function j(n){let e;try{e=JSON.parse(n)}catch{throw new Error("The voice config is not JSON. Choose the .onnx.json file that belongs to the Piper voice.")}let t=e,r=Number(t.audio?.sample_rate);if(!Number.isInteger(r)||r<8e3||r>48e3)throw new Error("The voice config has no usable audio.sample_rate.");if(t.phoneme_type!==void 0&&t.phoneme_type!=="espeak")throw new Error(`Only espeak phoneme voices are supported, this one uses ${String(t.phoneme_type)}.`);let o=t.phoneme_id_map;if(!o||typeof o!="object"||Array.isArray(o))throw new Error("The voice config has no phoneme_id_map.");let i={};for(let[c,p]of Object.entries(o))Array.isArray(p)&&p.length>0&&p.every(d=>Number.isInteger(d))&&(i[c]=p);for(let c of["^","$","_"])if(!i[c])throw new Error(`The voice config's phoneme_id_map lacks "${c}".`);let s=(c,p)=>typeof c=="number"&&Number.isFinite(c)?c:p,a=typeof t.espeak?.voice=="string"&&t.espeak.voice?t.espeak.voice:"en-us",l=typeof t.language?.name_english=="string"&&t.language.name_english?t.language.name_english:a;return{sampleRate:r,espeakVoice:a,language:l,phonemeIdMap:i,noiseScale:s(t.inference?.noise_scale,.667),lengthScale:s(t.inference?.length_scale,1),noiseW:s(t.inference?.noise_w,.8),numSpeakers:Math.max(1,Math.round(s(t.num_speakers,1)))}}function O(n){return Object.entries(n.phonemeIdMap).filter(([e])=>[...e].length===1&&!/[\r\n]/.test(e)).map(([e,t])=>`${e} ${t[0]}
`).join("")}function J(n){return{model_type:"vits",comment:"piper",language:n.language,voice:n.espeakVoice,has_espeak:"1",n_speakers:String(n.numSpeakers),sample_rate:String(n.sampleRate)}}function H(n){let e=t=>String(n.phonemeIdMap[t][0]);return{frontend:"characters",add_blank:"1",use_eos_bos:"1",bos_id:e("^"),eos_id:e("$"),blank_id:e("_"),pad_id:e("_")}}function I(n){return Object.keys(n.phonemeIdMap).filter(e=>[...e].length===1).join("")}function K(n){let e=new Set,t=0;for(;t<n.length;){let[r,o]=g(n,t),i=Math.floor(r/8),s=r%8;if(s===0){t=g(n,o)[1];continue}if(s===1){t=o+8;continue}if(s===5){t=o+4;continue}if(s!==2)throw new Error("The voice model is not an ONNX file.");let[a,l]=g(n,o),c=l+a;if(c>n.length)throw new Error("The voice model is not an ONNX file (truncated).");if(i===14){let p=X(n.subarray(l,c));p[1]!==void 0&&e.add(p[1])}t=c}return e}function L(n,e,t="espeak"){let r=K(n),o=new TextEncoder,i=[],s=J(e);if(t==="potoru"){let a=H(e),l=Object.keys(a).filter(c=>r.has(c));if(l.length)throw new Error(`This voice model already sets ${l.join(", ")}; the Potoru phonemizer needs a plain Piper export. Use the espeak phonemizer for it.`);Object.assign(s,a)}for(let[a,l]of Object.entries(s)){if(r.has(a))continue;let c=v([S(1,o.encode(a)),S(2,o.encode(l))]);i.push(S(14,c))}return i.length?v([n,...i]):n}function g(n,e){let t=0,r=1;for(let o=e;o<n.length&&o<e+10;o+=1){let i=n[o];if(t+=(i&127)*r,i<128)return[t,o+1];r*=128}throw new Error("The voice model is not an ONNX file.")}function X(n){let e={},t=0;for(;t<n.length;){let[r,o]=g(n,t);if(r%8!==2)return e;let[i,s]=g(n,o);e[Math.floor(r/8)]=new TextDecoder().decode(n.subarray(s,s+i)),t=s+i}return e}function C(n){let e=[],t=n;for(;t>=128;)e.push(t%128|128),t=Math.floor(t/128);return e.push(t),e}function S(n,e){return v([Uint8Array.from([...C(n*8+2),...C(e.length)]),e])}function v(n){let e=new Uint8Array(n.reduce((r,o)=>r+o.length,0)),t=0;for(let r of n)e.set(r,t),t+=r.length;return e}function F(n,e){let t=new Uint8Array(44+n.length*2),r=new DataView(t.buffer),o=(i,s)=>{for(let a=0;a<s.length;a+=1)r.setUint8(i+a,s.charCodeAt(a))};o(0,"RIFF"),r.setUint32(4,36+n.length*2,!0),o(8,"WAVE"),o(12,"fmt "),r.setUint32(16,16,!0),r.setUint16(20,1,!0),r.setUint16(22,1,!0),r.setUint32(24,e,!0),r.setUint32(28,e*2,!0),r.setUint16(32,2,!0),r.setUint16(34,16,!0),o(36,"data"),r.setUint32(40,n.length*2,!0);for(let i=0;i<n.length;i+=1){let s=Math.max(-1,Math.min(1,n[i]));r.setInt16(44+i*2,Math.round(s*32767),!0)}return t}var Y="Run `node scripts/setup-tts-wasm.mjs` with POTORU_SHERPA_TTS_DIR set (see authoring/fixtures/DEMO-SLIDE-STORY-REPORT.md).";function Q(n){return V(n.espeakVoice)}var Z=()=>import("./g2p-wasm-DI6V376L.mjs").then(n=>n.loadG2p());function $(n,e=y,t=Z){let r=j(n.config),o=n.phonemizer??Q(r);if(o==="potoru"&&!P(r.espeakVoice))throw new Error(`The Potoru phonemizer supports voices trained on espeak "en" or "id" only in this version (this voice uses espeak voice "${r.espeakVoice}"). Choose espeak.`);let i=I(r),s,a=()=>{let l=e();if(!s||s.engine!==l||s.generation!==l.generation){let c=new Uint8Array(n.model),p=L(c,r,o),d=p===c?c.slice():p,f={noiseScale:r.noiseScale,noiseScaleW:r.noiseW,lengthScale:r.lengthScale},u={generation:l.generation,engine:l,voice:l.load(d,O(r),f).then(h=>h.voice)};u.voice.catch(()=>{s===u&&(s=void 0)}),s=u}return s};return{provider:"sherpa",config:r,phonemizer:o,async synthesize(l,c={}){if(!/[\p{L}\p{N}]/u.test(b(l)))throw new Error("Nothing to say: the line has no words.");let p=b(l).replace(/\s+/g," ").trim(),d;if(o==="potoru"&&(d=E(await t(),l.replace(/\s+/g," ").trim(),c.lang,{symbols:i,espeakVoice:r.espeakVoice}),p=d.sentences.join(""),!p.trim()))throw new Error("Nothing to say: the Potoru G2P found no words it can read in this line.");let f=a(),u=await f.voice;if(c.signal?.aborted)throw new DOMException("Speech cancelled","AbortError");let{samples:h,sampleRate:w}=await f.engine.speak(u,p,0);if(!h.length)throw new Error("sherpa-onnx returned no audio for this line.");let q=F(h,w);return{blob:new Blob([q.buffer],{type:"audio/wav"}),durationSeconds:h.length/w,sampleRate:w,...d?{phonemes:d}:{}}},dispose(){let l=s;s=void 0,l&&l.voice.then(c=>l.engine.free(c),()=>{})}}}var ee=String.raw`
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
`,k=class{constructor(e){this.base=e}base;generation=0;worker;workerUrl;nextId=1;pending=new Map;load(e,t,r){let o=e.byteOffset===0&&e.byteLength===e.buffer.byteLength?e.buffer:e.slice().buffer;return this.call({type:"load",model:o,tokens:t,params:r},[o])}speak(e,t,r){return this.call({type:"speak",voice:e,text:t,speakerId:r})}free(e){this.worker&&this.call({type:"free",voice:e}).catch(()=>{})}call(e,t=[]){let r=this.ensureWorker(),o=this.nextId++;return new Promise((i,s)=>{this.pending.set(o,{resolve:i,reject:s}),r.postMessage({...e,id:o,base:this.base},t)})}ensureWorker(){if(this.worker)return this.worker;this.workerUrl=URL.createObjectURL(new Blob([ee],{type:"text/javascript"}));let e=new Worker(this.workerUrl);return e.onmessage=t=>{let{id:r,ok:o,result:i,error:s,fatal:a}=t.data,l=this.pending.get(r);if(this.pending.delete(r),o)l?.resolve(i);else{let c=/not installed|could not be loaded/.test(s??"");l?.reject(new Error(c?`${s} ${Y}`:s??"sherpa-onnx failed.")),a&&this.restart(new Error(s??"sherpa-onnx stopped."))}},e.onerror=t=>{t.preventDefault(),this.restart(new Error(`sherpa-onnx worker failed${t.message?`: ${t.message}`:"."}`))},this.worker=e,e}restart(e){this.worker?.terminate(),this.worker=void 0,this.workerUrl&&URL.revokeObjectURL(this.workerUrl),this.workerUrl=void 0,this.generation+=1;for(let t of this.pending.values())t.reject(e);this.pending.clear()}},N=new Map;function y(n){let e=new URL(n??"tts/",document.baseURI).href.replace(/\/?$/,"/"),t=N.get(e);return t||(t=new k(e),N.set(e,t)),t}function te(n){if(n.provider==="elevenlabs")return JSON.stringify(["elevenlabs",n.voiceId.trim(),n.modelId.trim()]);if(n.provider==="openai")return JSON.stringify(["openai",n.endpoint.trim().replace(/\/+$/,""),n.model.trim(),n.voice.trim()])}async function re(n,e,t){let r=new TextEncoder().encode(JSON.stringify([n,t??"",e])),o=await crypto.subtle.digest("SHA-256",r);return[...new Uint8Array(o)].map(i=>i.toString(16).padStart(2,"0")).join("")}function ne(n,e,t){return{provider:n.provider,async synthesize(r,o={}){let i;try{i=await re(e,r,o.lang);let a=await t.getLine(i);if(a)return{blob:a.blob,...a.durationSeconds!==void 0?{durationSeconds:a.durationSeconds}:{}}}catch{}let s=await n.synthesize(r,o);return i&&t.putLine(i,s.blob,s.durationSeconds).catch(()=>{}),s}}}function D(n,e={}){switch(n.provider){case"sherpa":{let t=n.catalog?.engine;return $(n,t?()=>y(t):y)}case"browser":return m(n);case"elevenlabs":case"openai":{let t=n.provider==="elevenlabs"?z(n):M(n),r=te(n);return e.cache&&r?ne(t,r,e.cache):t}}}function W(n){n.dispose?.()}var x=class{constructor(e=R()){this.store=e;U(()=>{this.reload()})}store;settings=[];synthesizers=new Map;problems=new Map;browser;listeners=new Set;catalogs=new Map;loading;generation=0;ready(){return this.loading??=this.reload(),this.loading}async reload(){let e=this.generation+=1,t=[];try{t=await this.store.list({localDefaults:!1})}catch{t=[]}if(e===this.generation){for(let r of this.synthesizers.values())W(r);this.settings=t,this.synthesizers=new Map,this.problems=new Map;for(let r of t)try{this.synthesizers.set(r.alias,D(r,{cache:this.store}))}catch(o){this.problems.set(r.alias,o instanceof Error?o.message:String(o))}for(let r of[...this.listeners])r()}}subscribe(e){return this.listeners.add(e),()=>{this.listeners.delete(e)}}get list(){return this.settings}setting(e){return this.settings.find(t=>t.alias===e)}problem(e){return this.problems.get(e)}get browserVoice(){if(this.browser===void 0)try{this.browser=m()}catch{this.browser=null}return this.browser??void 0}resolver(e){return{resolve:t=>this.synthesizers.get(t)??(e==="browser"?this.browserVoice:void 0)}}async assign(e){await this.store.put(e),await this.reload()}async unassign(e){await this.store.remove(e),await this.reload()}catalog(e){let t=new URL(e,document.baseURI).href,r=this.catalogs.get(t);return r||(r=A(t),r.catch(()=>this.catalogs.delete(t)),this.catalogs.set(t,r)),r}installed(){return this.store.installed().catch(()=>[])}async install(e,t,r={}){let o=await T(t,{...r.onProgress?{onProgress:r.onProgress}:{},...r.signal?{signal:r.signal}:{}}),i=oe(e,t,o.sha256);await this.store.putModel(i,o.model,o.config),_(),r.alias?await this.assign(B(r.alias,i,o.model,o.config,t)):await this.reload()}async useInstalled(e,t){let r=await this.store.getModel(t.key);if(!r)throw new Error(`${t.ref.name} is no longer downloaded.`);await this.assign(B(e,r.ref,r.model,r.config))}async uninstall(e){await this.store.removeModel(e),await this.reload()}};function oe(n,e,t){return{id:e.id,url:n.url,name:e.name,sha256:t,...e.version?{version:e.version}:{},...n.engine?{engine:n.engine}:{}}}function B(n,e,t,r,o){let i=(s,a)=>{try{return s?decodeURIComponent(new URL(s).pathname.split("/").pop()||a):a}catch{return a}};return{alias:n,provider:"sherpa",modelName:i(o?.model,`${e.id}.onnx`),configName:i(o?.config,`${e.id}.onnx.json`),model:t,config:r,catalog:e}}var G;function me(){return G??=new x,G}export{x as PlayerVoices,me as playerVoices};
