# Nova V7.0.0-alpha.9.3 Validation Notes

Static validation performed in the release workspace:

- all 24 JavaScript files pass `node --check`;
- package/Tauri JSON and Cargo TOML parse;
- Basic English structural regressions added for statement, question/request distinction, negation, coordination and previous-utterance reference;
- regressions assert the bootstrap does not label its state with hidden noun/pronoun/proper-name/agent/patient/grammar-rule concepts;
- Teacher regressions cover `0.5 / 1`, partial-credit derivation, grade label, exact strike span, per-span note, immutable originals, Inspector context and non-executable evidence;
- contradictory point/percentage grades are rejected;
- persistence regression removes alpha 9.3 fields from a synthetic alpha 9.2 mind and verifies defaults/normalization;
- Pass 9 Rust reasoning ownership remains unchanged;
- low-level alpha 6.1 atomic writer is intentionally not redesigned.

No Cargo/Rust toolchain is installed in this environment. The user's real Tauri build remains the authoritative compile and Rust-test execution check.
