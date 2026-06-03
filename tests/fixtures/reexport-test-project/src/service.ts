export function internalHelper(): void {
  console.log("helper");
}

export class Service {
  run(): void {
    internalHelper();
  }
}
