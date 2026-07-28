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
          // nie nadpisuj stanu gdy pobieranie/instalacja już trwa
          if (cancelled || !update?.available) return;
          setStatus((current) => {
            if (current === "downloading" || current === "ready") return current;
            setUpdateRef(update);
            setVersion(update.version);
            return "available";
          });
        })
        .catch((err) => {
          // cichy fail - brak sieci nie powinien blokować normalnego korzystania z apki
          console.warn("Sprawdzanie aktualizacji nie powiodło się:", err);
        });
    };

    runCheck();
    // apka bywa otwarta całymi dniami, a auto-update to główny kanał
    // dostarczania zmian - samo sprawdzenie przy starcie by ich nie złapało
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
      console.error("Błąd instalacji aktualizacji:", err);
      setErrorMsg(formatErrorMessage(err));
      setStatus("error");
    }
  };

  return { status, progress, version, errorMsg, downloadAndInstall };
}
