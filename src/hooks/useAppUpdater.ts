import { useEffect, useState } from "react";
import { check as checkForUpdate, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import type { UpdateStatus } from "../types";

export function useAppUpdater() {
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [progress, setProgress] = useState(0);
  const [version, setVersion] = useState<string | null>(null);
  const [updateRef, setUpdateRef] = useState<Update | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    checkForUpdate()
      .then((update) => {
        if (update?.available) {
          setUpdateRef(update);
          setVersion(update.version);
          setStatus("available");
        }
      })
      .catch((err) => {
        // cichy fail - brak sieci nie powinien blokować normalnego korzystania z apki
        console.warn("Sprawdzanie aktualizacji nie powiodło się:", err);
      });
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
      console.error("Błąd instalacji aktualizacji:", err);
      setErrorMsg(String(err));
      setStatus("error");
    }
  };

  return { status, progress, version, errorMsg, downloadAndInstall };
}
