// zeb/livegeo 0.1 — frontend hooks for live geospatial interactivity

const { useState, useEffect, useRef, useMemo, useCallback } = globalThis;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function normalizeAngle(angle) {
  let next = Number(angle) || 0;
  while (next < 0) next += 360;
  while (next >= 360) next -= 360;
  return next;
}

function interpolateAngle(from, to, factor) {
  let start = normalizeAngle(from);
  let end = normalizeAngle(to);
  let delta = end - start;
  if (delta > 180) delta -= 360;
  if (delta < -180) delta += 360;
  return normalizeAngle(start + delta * factor);
}

function defaultWindow(options) {
  const start = Number(options?.start ?? 0);
  const end = Number(options?.end ?? 1);
  return { start, end: end > start ? end : start + 1 };
}

export function usePlayback(options = {}) {
  const windowRange = useMemo(() => defaultWindow(options), [options?.start, options?.end]);
  const [time, setTime] = useState(
    Number.isFinite(options.initialTime) ? options.initialTime : windowRange.start
  );
  const [isPlaying, setIsPlaying] = useState(Boolean(options.autoplay));
  const [speed, setSpeed] = useState(Number.isFinite(options.speed) ? options.speed : 1);
  const frameRef = useRef(null);
  const lastRef = useRef(null);

  const pause = useCallback(() => setIsPlaying(false), []);
  const play = useCallback(() => setIsPlaying(true), []);
  const reset = useCallback(() => {
    setIsPlaying(false);
    setTime(windowRange.start);
  }, [windowRange.start]);
  const seek = useCallback((next) => {
    setTime(clamp(Number(next) || windowRange.start, windowRange.start, windowRange.end));
  }, [windowRange.end, windowRange.start]);

  useEffect(() => {
    if (!isPlaying) {
      if (frameRef.current) cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
      lastRef.current = null;
      return;
    }

    const scale = Number.isFinite(options.msPerSecond) ? options.msPerSecond : 1000;
    const loop = (now) => {
      const last = lastRef.current == null ? now : lastRef.current;
      lastRef.current = now;
      const deltaMs = now - last;
      setTime((current) => {
        const next = current + deltaMs * speed * scale / 1000;
        if (next >= windowRange.end) {
          if (options.loop) return windowRange.start;
          setIsPlaying(false);
          return windowRange.end;
        }
        return next;
      });
      frameRef.current = requestAnimationFrame(loop);
    };

    frameRef.current = requestAnimationFrame(loop);
    return () => {
      if (frameRef.current) cancelAnimationFrame(frameRef.current);
      frameRef.current = null;
      lastRef.current = null;
    };
  }, [isPlaying, options.loop, options.msPerSecond, speed, windowRange.end, windowRange.start]);

  const progress =
    (time - windowRange.start) / Math.max(1, windowRange.end - windowRange.start);

  return {
    time,
    setTime: seek,
    progress: clamp(progress, 0, 1),
    isPlaying,
    play,
    pause,
    reset,
    speed,
    setSpeed,
    start: windowRange.start,
    end: windowRange.end,
  };
}

export function useTrackPlayback(tracks, options = {}) {
  const trackList = Array.isArray(tracks) ? tracks : [];
  const windowRange = useMemo(() => {
    if (Number.isFinite(options.start) && Number.isFinite(options.end)) {
      return defaultWindow(options);
    }
    let start = Infinity;
    let end = -Infinity;
    trackList.forEach((track) => {
      const points = Array.isArray(track?.points) ? track.points : [];
      points.forEach((point) => {
        const at = Number(point?.at);
        if (!Number.isFinite(at)) return;
        if (at < start) start = at;
        if (at > end) end = at;
      });
    });
    if (!Number.isFinite(start) || !Number.isFinite(end)) return { start: 0, end: 1 };
    return { start, end: end > start ? end : start + 1 };
  }, [options.end, options.start, trackList]);

  const playback = usePlayback({
    start: windowRange.start,
    end: windowRange.end,
    initialTime: options.initialTime,
    autoplay: options.autoplay,
    speed: options.speed,
    msPerSecond: options.msPerSecond,
    loop: options.loop,
  });

  const entities = useMemo(() => {
    return trackList.map((track) => {
      const points = Array.isArray(track?.points) ? track.points : [];
      if (!points.length) {
        return {
          id: track?.id,
          position: [0, 0],
          bearing: 0,
          progress: 0,
          point: null,
        };
      }

      let previous = points[0];
      let current = points[points.length - 1];
      for (let index = 1; index < points.length; index += 1) {
        if (Number(points[index].at) >= playback.time) {
          previous = points[index - 1] || points[index];
          current = points[index];
          break;
        }
        previous = points[index];
      }

      const startAt = Number(previous?.at ?? playback.time);
      const endAt = Number(current?.at ?? startAt);
      const ratio = endAt > startAt
        ? clamp((playback.time - startAt) / (endAt - startAt), 0, 1)
        : 0;
      const route = [previous.position, current.position];
      const position = Tool.geo.interpolateRoute(route, ratio);
      const bearing = Tool.geo.bearing(previous.position, current.position);

      return {
        id: track?.id,
        position,
        bearing,
        progress: ratio,
        point: current,
      };
    });
  }, [playback.time, trackList]);

  return { ...playback, entities };
}

export function useTrackSmoothing(target, options = {}) {
  const factor = Number.isFinite(options.factor) ? options.factor : 0.18;
  const [state, setState] = useState(() => ({
    position: target?.position || [0, 0],
    bearing: normalizeAngle(target?.bearing || 0),
  }));
  const rafRef = useRef(null);

  useEffect(() => {
    const nextTarget = {
      position: target?.position || [0, 0],
      bearing: normalizeAngle(target?.bearing || 0),
    };
    const tick = () => {
      setState((current) => {
        const nextPosition = [
          current.position[0] + (nextTarget.position[0] - current.position[0]) * factor,
          current.position[1] + (nextTarget.position[1] - current.position[1]) * factor,
        ];
        const nextBearing = interpolateAngle(current.bearing, nextTarget.bearing, factor);
        return {
          position: nextPosition,
          bearing: nextBearing,
        };
      });
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    };
  }, [factor, target?.bearing, target?.position?.[0], target?.position?.[1]]);

  return state;
}

export function useMapFollow(target, options = {}) {
  const [viewState, setViewState] = useState(() => ({
    longitude: target?.[0] ?? options.longitude ?? 0,
    latitude: target?.[1] ?? options.latitude ?? 0,
    zoom: options.zoom ?? 14,
    pitch: options.pitch ?? 0,
    bearing: options.bearing ?? 0,
  }));

  useEffect(() => {
    if (!Array.isArray(target) || target.length < 2) return;
    setViewState((current) => ({
      ...current,
      longitude: Number(target[0]),
      latitude: Number(target[1]),
    }));
  }, [target?.[0], target?.[1]]);

  return [viewState, setViewState];
}

globalThis.usePlayback = usePlayback;
globalThis.useTrackPlayback = useTrackPlayback;
globalThis.useTrackSmoothing = useTrackSmoothing;
globalThis.useMapFollow = useMapFollow;
