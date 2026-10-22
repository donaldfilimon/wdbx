/**
 * The browser edition's declared losses.
 *
 * The native edition records the studio's episodic semantics in the WDBX v3
 * episode ledger. This edition persists to IndexedDB and owns no canonical
 * ledger at all, so it is a lossy projection of the native-owned ledger. A
 * consumer may expose a lossy projection, but it must declare what it drops:
 * this file is that declaration, and the native gate reads the same JSON so
 * the two editions cannot drift apart silently.
 */
import manifest from './canonical-loss.json';

/**
 * The closed vocabulary of what a studio projection can drop.
 *
 * The native edition declares the same ids in
 * `native/specimen-core/src/canonical/manifest.rs`, and the native gate reads
 * this edition's manifest into that same vocabulary, so an id one edition
 * declares is an id the other knows. This edition declares a subset of it:
 * it holds no ledger at all.
 */
export type LossId =
  | 'browser_edition'
  | 'calibration_evidence'
  | 'canonical_ledger'
  | 'content_resolution'
  | 'deletion_facts'
  | 'missing_content'
  | 'outcome_observation'
  | 'payload_commitment'
  | 'payload_content'
  | 'proposed_source'
  | 'record_age'
  | 'recorded_uncertainty'
  | 'regime_evidence'
  | 'replication_scope'
  | 'signature_evidence'
  | 'source_reputation'
  | 'unsigned_episodes';

/** One declared loss. */
export interface LossDeclaration {
  /** Which loss this is. */
  id: LossId;
  /** What this edition's projection does not carry. */
  dropped: string;
  /** Why leaving it out is honest rather than a hidden defect. */
  reason: string;
}

/** What one edition's projection drops relative to the canonical evidence. */
export interface LossManifest {
  /** The edition the manifest describes. */
  edition: string;
  /** The declared losses, in a stable order. */
  declarations: LossDeclaration[];
}

/** The browser edition's declaration, read from the manifest beside it. */
export const BROWSER_EDITION_LOSS = manifest as LossManifest;

/** The declaration for one loss, if the manifest names it. */
export function lossDeclaration(
  id: LossId,
  declaration: LossManifest = BROWSER_EDITION_LOSS,
): LossDeclaration | undefined {
  return declaration.declarations.find((item) => item.id === id);
}
