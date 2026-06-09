/**
 * Scenario runner: parses .feature files and executes them against the
 * in-browser WASM engine using the step definitions.
 */

import { parseFeature } from './parser.js';
import { stepDefinitions } from './steps.js';

class ExecutionContext {
  constructor() {
    this.calculationDate = null;
    this.parameters = {};
    this.result = null;
    this.error = null;
    this.executed = false;
  }
}

function findStep(text) {
  for (const def of stepDefinitions) {
    const match = def.pattern.exec(text);
    if (match) return { def, match };
  }
  return null;
}

/**
 * Run one feature file.
 *
 * @returns {{ feature: string, scenarios: Array<{name, passed, steps: Array<{keyword, text, status, error}>}> }}
 */
export async function runFeature(featureText, engine) {
  const parsed = parseFeature(featureText);
  const scenarios = [];

  for (const scenario of parsed.scenarios) {
    const ctx = new ExecutionContext();
    const stepResults = [];
    let failed = false;

    const allSteps = [...(parsed.background ?? []), ...scenario.steps];
    for (const step of allSteps) {
      if (failed) {
        stepResults.push({ ...step, status: 'overgeslagen', error: null });
        continue;
      }
      const found = findStep(step.text);
      if (!found) {
        stepResults.push({
          ...step,
          status: 'mislukt',
          error: `Geen stapdefinitie voor: ${step.text}`,
        });
        failed = true;
        continue;
      }
      try {
        await found.def.execute(ctx, engine, found.match, step);
        stepResults.push({ ...step, status: 'geslaagd', error: null });
      } catch (e) {
        stepResults.push({ ...step, status: 'mislukt', error: String(e?.message ?? e) });
        failed = true;
      }
    }

    scenarios.push({
      name: scenario.name,
      passed: !failed,
      steps: stepResults,
    });
  }

  return { feature: parsed.feature, scenarios };
}
