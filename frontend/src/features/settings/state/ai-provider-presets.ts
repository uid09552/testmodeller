/**
 * How the configured AI provider is described in Settings.
 *
 * The provider is configured in `.env` and passed to the backend by
 * compose.yaml (07-ai-integration.md); Settings only shows what is in effect.
 * The contract's `provider` enum names a protocol, not a product, so this
 * puts the product names in front of the user.
 */
import { AiProvider } from '../../../core/api/api.types';

/** The product a stored setting corresponds to. */
export type ProviderChoice = 'claude' | 'ollama' | 'openai-compatible' | 'none';

const LABELS: Record<ProviderChoice, string> = {
  claude: 'Claude (Anthropic)',
  ollama: 'Ollama',
  'openai-compatible': 'OpenAI-compatible',
  none: 'None — AI disabled',
};

/** The value to put in `TM_AI_PROVIDER` for each product. */
const ENV_VALUES: Record<ProviderChoice, string> = {
  claude: 'claude',
  ollama: 'ollama',
  'openai-compatible': 'openai-compatible',
  none: 'none',
};

/**
 * Which product a stored setting corresponds to.
 *
 * `local` and `openai-compatible` speak the same protocol, so the stored value
 * alone is ambiguous; port 11434 is taken as Ollama, which is the only thing
 * that endpoint is in practice.
 */
export function choiceFor(
  provider: AiProvider | undefined,
  baseUrl: string,
): ProviderChoice {
  switch (provider) {
    case 'anthropic':
      return 'claude';
    case 'local':
      return 'ollama';
    case 'openai-compatible':
      return baseUrl.includes(':11434') ? 'ollama' : 'openai-compatible';
    default:
      return 'none';
  }
}

export function providerLabel(choice: ProviderChoice): string {
  return LABELS[choice];
}

export function envValueFor(choice: ProviderChoice): string {
  return ENV_VALUES[choice];
}
