import { useEffect, useState } from "react";
import { check as checkForUpdate, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { UpdateStatus } from "../types";
import { formatErrorMessage } from "../utils/format";

const CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000; // 4h

export function useAppUpdater() {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [progress, setProgress] = useState(0);
  const [version, setVersion] = useState<string | null>(null);
  const [updateRef, setUpdateRef] = useState<Update | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const runCheck = () => {
      checkForUpdate()
        .then((update) => {
          // Do not overwrite state while a download or install is in progress.
          if (cancelled || !update?.available) return;
          setStatus((current) => {
            if (current === "downloading" || current === "ready") return current;
            setUpdateRef(update);
            setVersion(update.version);
            return "available";
          });
        })
        .catch((err) => {
          // Fails quietly: no network must not block normal use of the app.
          console.warn("Update check failed:", err);
        });
    };

    runCheck();
    // The app can stay open for days and auto-update is the main delivery
    // channel, so a startup-only check would miss releases.
    const interval = setInterval(runCheck, CHECK_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const downloadAndInstall = async () => {
    if (!updateRef) return;
    setStatus("downloading");
    setErrorMsg(null);
    let downloaded = 0;
    let total = 0;

    try {
      await updateRef.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) setProgress(Math.round((downloaded / total) * 100));
        } else if (event.event === "Finished") {
          setStatus("ready");
        }
      });
      await relaunch();
    } catch (err) {
      console.error("Update installation failed:", err);
      setErrorMsg(formatErrorMessage(err));
      setStatus("error");
    }
  };

  return { status, progress, version, errorMsg, downloadAndInstall };
}
