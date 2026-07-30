---
name: coding-standards
description: Coding standards to apply to Rust code
---

Code structure which MUST be followed:
- Keep close to the single-responsibility principle
- Use abstractions over capabilities
- Keep files shorter than 200 lines of code unless it makes things unconventionally verbose
- Prefer lean custom-built components over third-party libraries for small things

When writing `.slint` files:
- Component `property` and `callback` declarations must be listed with no more than one per line
- Component attributes must be listed with no more than one per line
- Component attributes defining callbacks should place the code on the lines after the opening brace
