from services.jobs import run_sync


class SyncWorker:
    def run(self) -> None:
        run_sync()
