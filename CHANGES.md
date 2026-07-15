# p99.Rust CHANGES <!-- omit in toc -->


## 0.0.3 - 14th July 2026

* standardised project boilerplate: (**.gitattributes**, **.gitignore**, **.vimrc**, **.vscode/settings.json**); project files (**CHANGES.md**, **EXAMPLES.md**, **NEWS.md**, **README.md**, **TODO.md**);


## 0.0.2 - 13th July 2026

* added opt-in crate feature **`"binary-scaling"`** that replaces integer division with $2^{32}$ fixed-point binary scaling for all integer-based percentile queries (`#value_at_p90()`, `#value_at_p95()`, `#value_at_p99()`, etc.), achieving a ~1.5x to 2x speedup with a small loss of accuracy;
* added **`"null-feature"`** -- a no-op feature that has no effect but simplifies driver scripts that conditionally pass features;


## 0.0.1 - 26th June 2026

FIRST PUBLIC RELEASE


<!-- ########################### end of file ########################### -->

