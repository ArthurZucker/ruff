# Modular model rules

Ruff includes preview rules for validating modular model files. These files are expected to be named `modular_<model>.py` and generate a `modeling_<model>.py` file when the rules are fixed.

The linter enforces:

- `RUF062` – unwrap inheritance to inline parent class members.
- `RUF063` – expand relative imports inside the file.
- `RUF064` – every class name must start with the model name derived from the file name.

## Running the rules

Use `ruff check` with the Transformers rules enabled:

```console
$ ruff check modular_llama.py --select RUF062,RUF063,RUF064
```

Applying the rules with `--fix` will emit a `modeling_<model>.py` file with the transformations applied:

```console
$ ruff check modular_llama.py --select RUF062,RUF063,RUF064 --fix > modeling_llama.py
```
