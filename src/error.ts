export class ScafalraError extends Error {
  constructor(message?: string) {
    super(message);
    this.name = 'ScafalraError';
  }

  static itemNotExists(name: string) {
    return new ScafalraError(`Not found: '${name}'.`);
  }

  static itemExists(name: string) {
    return new ScafalraError(`'${name}' already exists.`);
  }
}
