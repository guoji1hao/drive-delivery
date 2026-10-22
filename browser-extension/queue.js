export class UploadQueue {
  constructor(run, changed = () => {}, limit = 2) {
    this.run = run; this.changed = changed; this.limit = limit;
    this.jobs = []; this.paused = false; this.running = 0;
  }
  add(entries, providers, settings, bindings) {
    for (const entry of entries) for (const provider of providers) {
      this.jobs.push({ id: crypto.randomUUID(), ...entry, provider,
        destination: settings[provider].directory, binding: bindings[provider],
        publicShare: settings.publicShare, status: 'waiting', progress: 0, state: {} });
    }
    this.changed(); this.pump();
  }
  pause(value) { this.paused = value; this.changed(); this.pump(); }
  retry(job) {
    if (job.status !== 'failed') return;
    job.status = 'waiting'; job.error = ''; this.changed(); this.pump();
  }
  pump() {
    while (!this.paused && this.running < this.limit) {
      const job = this.jobs.find(item => item.status === 'waiting');
      if (!job) break;
      this.running++; job.status = 'running'; this.changed();
      Promise.resolve().then(() => this.run(job, progress => {
        job.progress = Math.min(100, Math.max(0, progress)); this.changed();
      })).then(result => {
        job.result = result; job.status = 'completed'; job.progress = 100;
      }).catch(error => {
        job.error = error.message || 'Upload failed'; job.status = 'failed';
      }).finally(() => { this.running--; this.changed(); this.pump(); });
    }
  }
}
