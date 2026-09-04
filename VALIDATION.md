# Validation record

Validated on September 4, 2026.

- TypeScript type checking: passed.
- Focused lint checks for studio, specimen runtime, and WebMCP adapter: passed.
- Production build: passed.
- Runtime tests: 13 passed, 57 assertions. Coverage includes arithmetic and parser failures, exact duplicate rejection, same-ID peers, preserved multi-entry edits, bind templates, bounded confidence, feedback deduplication, raft coverage/cancellation, pin retention, complete JSON round-trip, malformed nested data, last-peer detachment, Type A/B proposals, and bounded automata/PHAGY.
- Isolated Chrome acceptance: passed. Exercised arithmetic, contributor view, overlapping-feedback rejection, pinning, node creation, a taught response, combined settings/name save, IndexedDB reload, JSON download and restore, malformed import rejection, reference chapter navigation, and phone navigation.
- Browser sizes: 1568 × 1000 and 390 × 844. No horizontal document overflow in the studio or reader. No uncaught browser runtime errors in the acceptance run.
- WebMCP: the browser did not expose `document.modelContext`; native registration/execution remains unverified. The application feature-detects the optional API and retains all graphical workflows. This is not a claim of verified native WebMCP support.

## Design comparison

The full-screen generated studio and reader concepts were inspected before implementation. Rendered desktop and phone screenshots were inspected at their native size.

1. The cool gray left navigation, white canvas, and teal accent match the chosen direction.
2. The dominant composition remains a live dotted topology and adjacent node inspector.
3. The node field hierarchy, strength meter, jitter control, and edit action remain visible and operable.
4. The conversation, attribution feedback, and prompt form sit directly beneath the graph; the final desktop adjustment keeps the prompt within the 1000-pixel composition.
5. The specification reader retains a chapter rail, search, chapter counter, long-form text, and Markdown download. Actual specification wording is preserved rather than abbreviated to fit the concept.

Deliberate differences: responsive navigation becomes a drawer on phones; graph positions are computed from actual node count; the reference reader flows its full text vertically; the browser profile exposes real state and results instead of seeded response mockups. The visual synthesis is explicitly labeled as a fixed-feature visual study.

The accessibility pass followed the fetched Vercel Web Interface Guidelines: named controls, real buttons/links, associated form labels, visible focus, reduced-motion handling, error announcements, keyboard-supported dialogs, and URL-addressable reference views.

The broader specification remains distinct from this browser implementation. See RUNTIME-PROFILE.md for exact executable mechanisms and limits.
