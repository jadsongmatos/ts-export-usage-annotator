# ts-export-usage-annotator

Varre um projeto TypeScript, encontra exports e insere comentários indicando quais arquivos os utilizam.

## O que faz

Dado um projeto TypeScript, a ferramenta:

1. Descobre os arquivos `.ts`/`.tsx` via `tsconfig.json`
2. Coleta todos os exports de cada arquivo
3. Resolve os imports e descobre quem usa cada export
4. Insere comentários antes de cada export com a lista de consumidores

Resultado exemplo — `lib.ts` antes:

```ts
export function greet(name: string): string {
  return `Hello, ${name}!`;
}

export const VERSION = "1.0.0";
```

`lib.ts` depois:

```ts
/* Export greet usado por: src/app.ts */
export function greet(name: string): string {
  return `Hello, ${name}!`;
}

/* Export VERSION usado por: src/app.ts */
export const VERSION = "1.0.0";
```

## Instalação

Compilar localmente:

```sh
cargo build --release
```

Instalar globalmente (acessível de qualquer diretório):

```sh
cargo install --path .
```

Depois:

```sh
ts-export-usage-annotator --project /meu/projeto/tsconfig.json --dry-run
```

## Uso

### Simulação (não escreve nada)

```sh
ts-export-usage-annotator --project tsconfig.json --dry-run
```

Mostra quais arquivos seriam alterados sem modificar nada.

### Escrita em diretório de saída

```sh
ts-export-usage-annotator --project tsconfig.json --write --out-dir .annotated
```

Cria os arquivos anotados dentro de `.annotated/`, preservando os originais.

### Escrita in-place

```sh
ts-export-usage-annotator --project tsconfig.json --write --in-place
```

Sobrescreve os arquivos originais com as anotações.

## Opções

| Flag | Descrição |
|------|-----------|
| `--project`, `-p` | Caminho para o `tsconfig.json` (padrão: `tsconfig.json`) |
| `--dry-run` | Apenas simula; não escreve arquivos (modo padrão) |
| `--write` | Ativa escrita dos arquivos anotados |
| `--in-place` | Sobrescreve os arquivos originais |
| `--out-dir` | Diretório de saída (usado com `--write`, sem `--in-place`) |

## Funcionalidades

- **Named imports**: `import { X } from "./y"`
- **Default imports**: `import X from "./y"`
- **Namespace imports**: `import * as X from "./y"` — expande para todos os exports do módulo
- **Re-exports**: `export { X } from "./y"` — rastreia uso até o arquivo original
- **Path aliases**: suporte a `baseUrl` e `paths` do `tsconfig.json`
- **tsconfig com comentários**: aceita `/* */` e `//` no tsconfig.json
- **Exclude inteligente**: diretórios como `node_modules`, `dist`, `coverage` são pulados automaticamente (não entra nem no diretório)

## Testando com os fixtures

```sh
ts-export-usage-annotator --project tests/fixtures/ts-test-project/tsconfig.json --dry-run
ts-export-usage-annotator --project tests/fixtures/ns-test-project/tsconfig.json --dry-run
ts-export-usage-annotator --project tests/fixtures/reexport-test-project/tsconfig.json --dry-run
```
