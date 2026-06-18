# Senior Rust Developer - Clean Code Agent Guidelines

## Core Principles

### 1. Code Quality Standards

- **Readability First**: Code should be self-documenting and easy to understand
- **Simplicity**: Favor simple solutions over complex ones
- **Consistency**: Follow established patterns and conventions throughout the codebase
- **Testability**: Write code that is easy to test and maintain

### 2. Rust-Specific Clean Code Practices

#### Naming Conventions

- Use `snake_case` for functions and variables
- Use `PascalCase` for types, structs, and enums
- Use `SCREAMING_SNAKE_CASE` for constants
- Choose descriptive names that reveal intent
- Avoid abbreviations unless widely understood

#### Function Design

- Keep functions small and focused (single responsibility)
- Prefer explicit returns over implicit ones for clarity
- Use meaningful parameter names
- Limit function parameters to 3-4 when possible
- Use result types (`Result<T, E>`) for error handling

#### Struct and Organization

- Group related fields together in structs
- Use `#[derive(Debug, Clone, PartialEq)]` judiciously
- Implement `Drop` only when necessary
- Prefer composition over inheritance

#### File and Module Organization

- **One struct/enum per file**: Each struct or enum should be in its own file to maintain clear separation of concerns
- **Logical module grouping**: Group related types into modules based on domain responsibility (e.g., `config/area/` for area-specific types, `config/layout/`
  for layout-specific types)
- **Module structure**: Create a `mod.rs` in each module directory with module declarations and `pub use` re-exports for a clean API
- **Naming conventions**: Avoid redundant prefixes in filenames when the module name already provides context (e.g., `area_config.rs` → `config.rs` in
  `config/area/` module)
- **Clear separation**: Ensure modules have distinct responsibilities - if types belong to different domains, separate them into different modules

### 3. Error Handling

- Use `Result<T, E>` for recoverable errors
- Use `Option<T>` for values that may be absent
- Create custom error types with `thiserror` or `anyhow`
- Handle errors at the appropriate level
- Avoid using `panic!()` in production code

### 4. Memory Management

- Let Rust's ownership system manage memory
- Use references (`&`) when you don't need ownership
- Consider `Cow<str>` for string handling when appropriate
- Be mindful of lifetimes and avoid unnecessary allocations

### 5. Concurrency

- Use channels for message passing
- Prefer `Arc<Mutex<T>` over `RwLock<T>` when write operations are rare
- Use async/await for I/O-bound operations
- Avoid blocking operations in async contexts

## Code Review Checklist

### Before Submitting Code

- [ ] Code follows Rust naming conventions
- [ ] Functions are small and focused
- [ ] Error handling is comprehensive
- [ ] No unused dependencies or imports
- [ ] Tests cover critical paths
- [ ] Documentation is clear and concise
- [ ] Performance considerations are addressed
- [ ] Security implications are considered

### During Code Review

- [ ] Code is readable and maintainable
- [ ] Abstractions are appropriate
- [ ] No code duplication
- [ ] Proper use of Rust features (iterators, patterns, etc.)
- [ ] Memory usage is efficient
- [ ] Error messages are helpful

## Testing Guidelines

### Unit Tests

- Test public API behavior
- Use descriptive test names
- Test both success and failure cases
- Use `#[should_panic]` for expected panics
- Mock external dependencies when necessary

### Integration Tests

- Test component interactions
- Use realistic test data
- Test error propagation
- Verify performance characteristics

### Documentation Tests

- Include examples in doc comments
- Test code examples with `cargo test --doc`
- Ensure examples compile and run

## Performance Considerations

### Optimization Guidelines

- Profile before optimizing
- Use `#[inline]` judiciously
- Consider `Box<T>` for large types
- Avoid unnecessary allocations

### Memory Efficiency

- Use stack allocation when possible
- Use `String::from` vs `to_string` appropriately
- Be mindful of string allocations in loops

## Tooling and Workflow

### Development Tools

- Use `rustfmt` for consistent formatting
- Use `clippy` for linting and suggestions
- Use `cargo-audit` for security checks
- Use `cargo-deny` for dependency checking

### Git Workflow

- Write clear, descriptive commit messages
- Use conventional commit format
- Keep commits small and focused
- Review own code before requesting review

## Learning and Growth

### Code Quality Metrics

- Monitor cyclomatic complexity
- Track test coverage
- Measure performance regressions
- Review security vulnerabilities

## Project-Specific Requirements

### Requirements

1. Widgets are individual crates (`plugins/<name>`)
2. Services are individual crates (`services/<name>`)
3. Widget (View) and Service (Business Logic) must be separated
4. When Widget and Service need shared structs or enums, these belong in a separate crate (`model/<name>`)
5. For Services, use the `service_plugin!(MyService);` macro
6. For Widgets, use the `widget_plugin!(MyWidget);` macro
7. Implement the Service struct in `service.rs` and implement the traits `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`, `AsRef<
   Option<FfiCoreContext>>`
8. Implement the Widget struct in `widget.rs` and implement the traits `MessageHandler`, `MessageBroadcaster`, `PluginMetaGetter`,
   `AsRef<Option<FfiCoreContext>>`
9. In `model`, implement Actions and Message formats
10. When a Widget needs a config, implement a dedicated struct in `config.rs` with a `parse` method
11. FFI-relevant types in `model` crates must carry `#[stabby::stabby]`
12. Services must use `tokio::sync::mpsc` instead of `std::sync::mpsc` and spawn async tasks via `PluginExecutor`
13. Widgets must use `glib::MainContext::spawn_local` for GTK updates and `tokio::sync::mpsc` for message reception
14. Polling loops (`timeout_add_local`) are forbidden; use event-driven `recv().await` instead

### Examples

- Example for Service: services/app-launcher
    - service.rs: Service struct (+ implementation of MessageHandler, MessageBroadcaster, PluginMetaGetter, AsRef<Option<FfiCoreContext>>)
    - lib.rs: Implement service_plugin! macro
- Example for Widget: plugins/app-launcher
    - config.rs: Struct for the config file part (+ parsing)
    - widget.rs: Widget struct (+ implementation of MessageHandler, MessageBroadcaster, PluginMetaGetter, AsRef<Option<FfiCoreContext>>)
    - lib.rs: Implement widget_plugin! macro
- Example for Model: model/app-launcher
    - Message system topics
    - Enums for Actions
    - Structs for message system payload
    - All FFI-relevant types with `#[stabby::stabby]`

### Rust Implementation Standards

- **Rust Edition 2024**: Use latest edition features
- **Modern Versions**: Keep dependencies updated to modern versions
- **Idiomatic Rust**: Follow Rust best practices and patterns
- **Panic-Free Code**: Avoid `unwrap()`, `expect()`, and panicking code
- **English Comments**: All source code comments in English
- **No Abbreviations**: Use descriptive variable names without abbreviations

### Documentation Standards

- **Type Documentation**: All public enums and structs must have rustdoc comments describing their purpose
- **Enum Variants**: Each enum variant must be documented with a brief description of its meaning
- **Struct Fields**: Each field in a struct must be documented with a description of its purpose and any relevant details
- **Documentation Format**: Use `///` for rustdoc comments, follow rustdoc conventions
- **Language**: All documentation must be in English
- **Clarity**: Documentation should be clear, concise, and focus on semantic meaning

### Import Organization

- **Individual Imports**: One import per line
- **No Star Imports**: Except for preludes (e.g., `gtk4::prelude::*`)
- **No Import Grouping**: Keep imports separate and ungrouped
- **No Import Comments**: Don't comment import statements
- **Macro Usage**: Use `debug!` instead of `tracing::debug!` with proper imports

### Dependencies

- `thiserror`: Internal error types
- `miette`: User-facing error types
- `clap`: Command line argument parsing
- `gtk4`: GTK4 framework for UI widgets
- `glib`: GLib utilities and patterns
- `stabby`: ABI-stable types and FFI trait objects
- `tokio`: Async runtime for services
- `libloading`: Dynamic library loading (used with stabby ABI verification)

### Key Features to Implement

- **Smart Pointers**: Use `Rc`, `RefCell`, `Box<dyn Fn>`, `Weak`, `glib::clone`
- **Type Safety**: Leverage GTK4 type systems
- **Error Handling**: Integrate miette and thiserror
- **Async I/O**: Use `tokio::sync::mpsc` for message passing; spawn async tasks via `PluginExecutor`
- **ABI Stability**: Use `#[stabby::stabby]` for FFI-relevant types; use `stabby::libloading::StabbyLibrary` for verified plugin loading
- **Zero-Copy Messages**: Pass messages via raw pointers with `type_id` for type-safe downcasting (no serialization overhead)

### Testing Requirements

- **Idiomatic Tests**: Use idiomatic Rust testing patterns
- **Inline Tests**: Keep tests in the same file as the source code
- **Comprehensive Coverage**: Test both success and error paths

## Resources

### Documentation

- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [GTK4 Rust Documentation](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/)

### Tools and Libraries

- `clippy` - Rust linter
- `rustfmt` - Code formatter
- `cargo-audit` - Security audit
- `thiserror` - Error handling
- `miette` - User-friendly error reporting
- `anyhow` - Error handling
- `tokio` - Async runtime
- `serde` - Serialization
- `gtk4` - GTK4 bindings
- `clap` - CLI argument parsing
- `stabby` - ABI-stable FFI types and trait objects

---

*This guide should be updated regularly to reflect best practices and team experience.*