export class InterfaceError extends Error {
  readonly code: "empty_id" | "empty_revision";

  constructor(code: "empty_id" | "empty_revision") {
    super(code);
    this.code = code;
    this.name = "InterfaceError";
  }
}

