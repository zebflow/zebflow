//! RWE TSX compile benchmark.
//!
//! Measures compile-time cost for two TSX paths:
//! 1. `compile_template` (engine parses TSX internally)
//! 2. `lower_tsx_source_to_parts + compile_direct_parts`

use std::time::{Duration, Instant};

use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::tsx_frontend::lower_tsx_source_to_parts;
use zebflow::rwe::{
    NoopReactiveWebEngine, ReactiveWebEngine, ReactiveWebOptions, StyleEngineMode, TemplateSource,
};

fn main() {
    let engine = NoopReactiveWebEngine;
    let language = NoopLanguageEngine;
    let options = ReactiveWebOptions {
        style_engine: StyleEngineMode::TailwindLike,
        ..Default::default()
    };

    let cases = [
        ("fixture", fixture_tsx(), 320usize),
        ("small", generated_tsx("small.page", 24), 320usize),
        ("medium", generated_tsx("medium.page", 160), 160usize),
        ("large", generated_tsx("large.page", 900), 36usize),
    ];

    println!("RWE Compile Benchmark");
    println!("Mode: tsx baseline vs tsx manual-direct");
    println!();
    println!(
        "{:<8} {:>8} {:>18} {:>18} {:>10}",
        "case", "iters", "baseline (us)", "manual-direct (us)", "dir/base"
    );
    println!("{}", "-".repeat(72));

    for (case_name, tsx_source, iters) in cases {
        warmup(&engine, &language, &options, case_name, &tsx_source);

        let baseline = bench_baseline(&engine, &language, &options, case_name, &tsx_source, iters);
        let manual =
            bench_manual_direct(&engine, &language, &options, case_name, &tsx_source, iters);

        let baseline_us = dur_avg_us(baseline, iters);
        let manual_us = dur_avg_us(manual, iters);
        let ratio = if baseline_us > 0.0 {
            manual_us / baseline_us
        } else {
            0.0
        };

        println!(
            "{:<8} {:>8} {:>18.2} {:>18.2} {:>9.2}x",
            case_name, iters, baseline_us, manual_us, ratio
        );
    }
}

fn fixture_tsx() -> String {
    include_str!("../../bench-fixtures/rwe/compile_compare.tsx").to_string()
}

fn generated_tsx(page_id: &str, cards: usize) -> String {
    let mut blocks = String::new();
    for i in 0..cards {
        let key = format!("c{i:04}");
        blocks.push_str(&format!(
            r#"
      <article className=\"rounded-xl border border-gray-200 p-4 hover:border-red-700 transition-all\">
        <h3 className=\"text-base font-bold text-gray-900\">{{input.cards.{key}.title}}</h3>
        <p className=\"text-sm text-gray-600 mt-2\">{{input.cards.{key}.desc}}</p>
        <button onClick=\"card.toggle\" className=\"mt-3 px-3 py-1 rounded bg-gray-900 text-white text-xs font-mono\">Toggle</button>
      </article>"#
        ));
    }

    format!(
        r#"export const page = {{
  head: {{
    title: "{{{{input.seo.title}}}}",
    description: "{{{{input.seo.description}}}}"
  }},
  html: {{
    lang: "en"
  }},
  body: {{
    className: "min-h-screen bg-zinc-50 text-gray-900 font-sans"
  }},
  navigation: "history"
}};

export const app = {{
  state: {{
    pageTitle: \"Bench\",
    counter: 0,
    lastToggleAt: 0,
    badge: \"warm\"
  }},
  actions: {{
    \"card.toggle\": (ctx) => {{
      const next = (ctx.get(\"state.counter\") || 0) + 1;
      ctx.set(\"state.counter\", next);
      ctx.set(\"state.lastToggleAt\", Date.now());
      return \"state.counter\";
    }}
  }},
  memo: {{
    pageTitleUpper: (ctx) => String(ctx.get(\"state.pageTitle\") || \"\").toUpperCase(),
    counterLabel: (ctx) => `count:${{ctx.get(\"state.counter\") || 0}}`
  }},
  effect: {{
    syncBadge: {{
      deps: [\"state.counter\"],
      run: (ctx) => {{
        const count = Number(ctx.get(\"state.counter\") || 0);
        ctx.set(\"state.badge\", count > 9 ? \"hot\" : \"warm\");
      }}
    }}
  }}
}};

export default function Page(input) {{
  return (
    <Page>
        <main className=\"mx-auto max-w-6xl px-6 py-10\">
          <header className=\"mb-8\">
            <h1 className=\"text-3xl font-black tracking-tight\">{{input.pageTitle}}</h1>
            <p className=\"text-sm text-gray-600 mt-1\">page: {page_id}</p>
          </header>
          <section className=\"grid md:grid-cols-2 xl:grid-cols-3 gap-4\">{blocks}
          </section>
        </main>
    </Page>
  );
}}
"#
    )
}

fn warmup(
    engine: &NoopReactiveWebEngine,
    language: &NoopLanguageEngine,
    options: &ReactiveWebOptions,
    case_name: &str,
    tsx_source: &str,
) {
    for _ in 0..16 {
        compile_baseline(engine, language, options, case_name, tsx_source);
        compile_manual_direct(engine, language, options, case_name, tsx_source);
    }
}

fn bench_baseline(
    engine: &NoopReactiveWebEngine,
    language: &NoopLanguageEngine,
    options: &ReactiveWebOptions,
    case_name: &str,
    tsx_source: &str,
    iters: usize,
) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        compile_baseline(engine, language, options, case_name, tsx_source);
    }
    start.elapsed()
}

fn bench_manual_direct(
    engine: &NoopReactiveWebEngine,
    language: &NoopLanguageEngine,
    options: &ReactiveWebOptions,
    case_name: &str,
    tsx_source: &str,
    iters: usize,
) -> Duration {
    let start = Instant::now();
    for _ in 0..iters {
        compile_manual_direct(engine, language, options, case_name, tsx_source);
    }
    start.elapsed()
}

fn compile_baseline(
    engine: &NoopReactiveWebEngine,
    language: &NoopLanguageEngine,
    options: &ReactiveWebOptions,
    case_name: &str,
    tsx_source: &str,
) {
    let template = TemplateSource {
        id: format!("bench.{case_name}.tsx"),
        source_path: None,
        markup: tsx_source.to_string(),
    };
    let _ = engine
        .compile_template(&template, language, options)
        .expect("baseline tsx compile");
}

fn compile_manual_direct(
    engine: &NoopReactiveWebEngine,
    language: &NoopLanguageEngine,
    options: &ReactiveWebOptions,
    case_name: &str,
    tsx_source: &str,
) {
    let lowered = lower_tsx_source_to_parts(tsx_source).expect("tsx lower");
    let _ = engine
        .compile_direct_parts(
            &format!("bench.{case_name}.tsx.direct"),
            None,
            &lowered.html_template,
            lowered.control_script_source.as_deref(),
            options,
            language,
        )
        .expect("manual direct compile");
}

fn dur_avg_us(dur: Duration, iters: usize) -> f64 {
    (dur.as_secs_f64() * 1_000_000.0) / iters as f64
}
