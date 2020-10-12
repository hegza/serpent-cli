## Developer notes
This CLI includes the `serpent` library from an adjacent directory. This directory must exist
parallel to the serpent tool like so:
```
- serpent/
- serpent-cli/
```

## Examples
### Transpile a module into target directory
`serpent tp examples/black_scholes -o black-scholes-serpent --emit-manifest`

### Show intermediate transpilation results for a line
```sh
serpent steps examples/black_scholes/ -l 50 -f examples/black_scholes/black_scholes_dp.py
```

... outputs:

```sh
d1 = (np.log(S / K) + (r - q + 0.5 * sigma ** 2) * T) / (sigma * np.sqrt(T))

...{Python AST}

...{Rust AST}

...{Transpiled Rust}
```
