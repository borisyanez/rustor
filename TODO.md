# TODO: PHPStan Compatibility — Next Steps

Recommendations from codebase analysis of the PHPStan compatibility strategy (2026-02-22).

## Priority 1: PHPDoc Parsing for `@param`, `@return`, `@var`

**Impact:** High — unlocks accurate analysis for the majority of annotated PHP codebases.

PHPStan's real power at levels 5+ comes from docblock type information. Many large codebases (especially pre-PHP 7.4) rely on PHPDoc annotations as the primary source of type data. Without parsing these, Rustor will:
- Produce false positives on well-annotated code
- Miss errors that PHPDoc types would catch
- Undermine the "drop-in replacement" claim for teams using PHPStan at level 5+

### Scope
- [ ] Parse `@param Type $name` annotations
- [ ] Parse `@return Type` annotations
- [ ] Parse `@var Type` annotations
- [ ] Integrate parsed types into the symbol table and type checker
- [ ] Support common PHPStan extensions: `@phpstan-param`, `@phpstan-return`, `@phpstan-var`
- [ ] Support `@phpstan-type` and `@phpstan-import-type` aliases
- [ ] Handle generic syntax in docblocks (e.g., `array<string, int>`, `Collection<User>`)

## Priority 2: Framework Extension Stubs

**Impact:** High — removes the biggest adoption blocker for framework-heavy codebases.

Many teams use PHPStan extensions (`phpstan-doctrine`, `phpstan-symfony`, `phpstan-laravel`, `phpstan-phpunit`) that teach PHPStan about framework magic (e.g., Eloquent dynamic properties, Doctrine repository generics, Symfony container types). Without extension parity, Rustor reports false positives on any framework-heavy codebase.

Full extension support is a large effort, but shipping pre-built type stubs would cover ~80% of framework users with much less work.

### Scope
- [ ] Ship type stubs for Laravel (Eloquent models, facades, collections)
- [ ] Ship type stubs for Symfony (container, form, console)
- [ ] Ship type stubs for Doctrine (EntityManager, repositories, QueryBuilder)
- [ ] Ship type stubs for PHPUnit (TestCase, mock builder)
- [ ] Support loading custom stub files from `.rustor.toml` config

## Priority 3: Differential Test Harness (PHPStan vs Rustor)

**Impact:** Medium — catches divergences before users do, builds confidence in compatibility claims.

Levels 7–10 are implemented but unvalidated against PHPStan's actual output. Users at level 8+ are the most rigorous about static analysis and will notice discrepancies quickly. A systematic comparison tool would:
- Quantify exact compatibility percentage per level
- Catch regressions automatically in CI
- Provide evidence for compatibility claims

### Scope
- [ ] Build a test runner that executes both PHPStan and Rustor on a shared corpus
- [ ] Compare outputs: match by file, line, error identifier, and message
- [ ] Generate a compatibility report (matched, Rustor-only, PHPStan-only)
- [ ] Add to CI to prevent regressions
- [ ] Curate a test corpus covering levels 0–10 patterns
- [ ] Track compatibility percentage over time

## Priority 4: Array Shape and Callable Signature Validation

**Impact:** Medium — matters increasingly at higher levels and in modern typed PHP codebases.

### Scope
- [ ] Support `array{key: Type, ...}` shape syntax in type system
- [ ] Validate array access against known shapes
- [ ] Support `callable(Type): ReturnType` signature validation
- [ ] Integrate with PHPDoc parsing (Priority 1) for docblock-declared shapes
