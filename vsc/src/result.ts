export type Ok<T> = { Ok: T };
export type Err<E = unknown> = { Err: E };
export type Result<T, E = unknown> = Ok<T> | Err<E>;

export function isOk<T>(value: unknown): value is Ok<T> {
  const keys = Object.keys(value as object);

  return keys.includes("Ok") && keys.length === 1;
}

export function isErr<E = unknown>(value: unknown): value is Err<E> {
  const keys = Object.keys(value as object);

  return keys.includes("Err") && keys.length === 1;
}

export function ifOk<T, F extends (value: T) => void | Promise<void>>(
  value: Result<T>,
  fn: F
): ReturnType<F> | void {
  if (isOk(value)) {
    return fn(value.Ok) as ReturnType<F>;
  }

  return undefined as ReturnType<F>; // Explicitly cast `undefined` to match the type
}

export function ifOkOr<
  T,
  F extends (value: T) => void | Promise<void>,
  E extends (value: unknown) => void | Promise<void>
>(value: Result<T>, okFn: F, errFn: E) {
  if (isOk(value)) {
    return okFn(value.Ok);
  } else {
    return errFn(value.Err);
  }
}

export function mapResult<T, U, E = unknown>(
  result: Result<T, E>,
  fn: (value: T) => U
): Result<U, E> {
  if (isOk(result)) {
    return { Ok: fn(result.Ok) };
  }
  return result;
}
