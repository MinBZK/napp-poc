/**
 * Step definitions for the NAPP scenario suite — the in-browser counterpart
 * of backend/tests/bdd/steps/. Patterns must match scenarios/*.feature.
 */

export function parseValue(str) {
  if (str === 'true') return true;
  if (str === 'false') return false;
  if (str === 'null') return null;
  if (/^-?\d+$/.test(str)) {
    const n = parseInt(str, 10);
    if (Number.isSafeInteger(n)) return n;
  }
  if (/^-?\d+\.\d+$/.test(str)) {
    const f = parseFloat(str);
    if (Number.isFinite(f)) return f;
  }
  return str;
}

const WPP_ID = 'wet_op_de_politieke_partijen';

function getOutput(ctx, name) {
  if (!ctx.result || !ctx.result.outputs) {
    throw new Error(
      `Geen outputs beschikbaar (${ctx.error ? `fout: ${ctx.error}` : 'niet uitgevoerd'})`,
    );
  }
  return ctx.result.outputs[name];
}

function assertOutput(ctx, name, expected) {
  const actual = getOutput(ctx, name);
  const equal =
    actual === expected ||
    (typeof actual === 'number' &&
      typeof expected === 'number' &&
      Math.abs(actual - expected) < 1e-9);
  if (!equal) {
    throw new Error(
      `Verwacht dat output "${name}" gelijk is aan ${JSON.stringify(expected)}, maar kreeg: ${JSON.stringify(actual)}`,
    );
  }
}

export const stepDefinitions = [
  {
    pattern: /^the calculation date is "([^"]+)"$/,
    execute: (ctx, _engine, match) => {
      ctx.calculationDate = match[1];
    },
  },
  {
    pattern: /^an application with the following data:$/,
    execute: (ctx, _engine, _match, step) => {
      if (!step.dataTable) return;
      for (const row of step.dataTable) {
        ctx.parameters[row[0]] = parseValue(row[1] ?? '');
      }
    },
  },
  {
    pattern: /^the subsidiebesluit is executed$/,
    execute: (ctx, engine) => {
      try {
        ctx.result = engine.executeMultiple(
          WPP_ID,
          ['subsidie_toegekend', 'subsidiebedrag'],
          ctx.parameters,
          ctx.calculationDate ?? '2026-06-01',
        );
        ctx.error = null;
      } catch (e) {
        ctx.error = e;
        ctx.result = null;
      }
      ctx.executed = true;
    },
  },
  {
    pattern: /^the subsidie is toegekend$/,
    execute: (ctx) => assertOutput(ctx, 'subsidie_toegekend', true),
  },
  {
    pattern: /^the subsidie is afgewezen$/,
    execute: (ctx) => assertOutput(ctx, 'subsidie_toegekend', false),
  },
  {
    pattern: /^the subsidiebedrag is "(-?\d+)" eurocent$/,
    execute: (ctx, _engine, match) =>
      assertOutput(ctx, 'subsidiebedrag', parseInt(match[1], 10)),
  },
  {
    pattern: /^a betaalopdracht of "(-?\d+)" eurocent is required$/,
    execute: (ctx, _engine, match) => {
      assertOutput(ctx, 'betaalopdracht_vereist', true);
      assertOutput(ctx, 'betaalopdracht_bedrag', parseInt(match[1], 10));
    },
  },
  {
    pattern: /^no betaalopdracht is required$/,
    execute: (ctx) => assertOutput(ctx, 'betaalopdracht_vereist', false),
  },
  {
    pattern: /^the bezwaartermijn is "(\d+)" weken$/,
    execute: (ctx, _engine, match) =>
      assertOutput(ctx, 'bezwaartermijn_weken', parseInt(match[1], 10)),
  },
  {
    pattern: /^motivering is vereist$/,
    execute: (ctx) => assertOutput(ctx, 'motivering_vereist', true),
  },
];
