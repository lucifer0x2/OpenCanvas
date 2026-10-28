export class PipelineError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
    this.name = "PipelineError";
  }
}
