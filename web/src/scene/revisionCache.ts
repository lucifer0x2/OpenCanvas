/** Matching FIFO retention on both ends avoids unbounded history in a long-lived tab. */
export class RevisionCache<T> {
  private projectId = "";
  private revisions = new Map<number, T>();

  constructor(private limit = 4) {}

  get(projectId: string, revision: number): T | undefined {
    this.select(projectId);
    return this.revisions.get(revision);
  }

  set(projectId: string, revision: number, value: T) {
    this.select(projectId);
    this.revisions.set(revision, value);
    if (this.revisions.size > this.limit)
      this.revisions.delete(this.revisions.keys().next().value!);
  }

  clear() {
    this.projectId = "";
    this.revisions.clear();
  }

  private select(projectId: string) {
    if (this.projectId !== projectId) {
      this.clear();
      this.projectId = projectId;
    }
  }
}
