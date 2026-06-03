export function greet(name: string): string {
  return `Hello, ${name}!`;
}

export const VERSION = "1.0.0";

export type User = {
  id: number;
  name: string;
};

export default function main() {
  console.log(greet("world"));
}
