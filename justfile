test:
    cargo test --features=test -p drax

@example project:
    echo "<--====-->"
    echo -e "Running example \`\\033[36m{{project}}\\033[0m\`."
    echo "<--====-->"
    cargo run -p {{project}}

publish:
    cargo publish -p drax --all-features
