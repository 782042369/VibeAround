(() => {
  "use strict";

  const releaseBase =
    "https://github.com/782042369/VibeAround/releases/latest/download";
  const proxyBase = "https://gh-proxy.org/";
  const downloads = {
    windows: `${proxyBase}${releaseBase}/VibeWbz_windows-x64-installer-nsis.exe`,
    macos: `${proxyBase}${releaseBase}/VibeWbz_macos-universal.dmg`,
  };

  const title = document.querySelector("#title");
  const status = document.querySelector("#status");
  const progress = document.querySelector("#progress");
  const retry = document.querySelector("#retry");

  function detectPlatform() {
    const platform = navigator.userAgentData?.platform || navigator.platform || "";
    const userAgent = navigator.userAgent || "";
    const signature = `${platform} ${userAgent}`;

    if (/Android|iPhone|iPad|iPod/i.test(userAgent)) {
      return null;
    }
    if (/Windows|Win32|Win64|WinCE/i.test(signature)) {
      return "windows";
    }
    if (/macOS|Macintosh|MacIntel|MacPPC|Mac68K/i.test(signature)) {
      return "macos";
    }
    return null;
  }

  function showUnsupported() {
    title.textContent = "当前系统暂不支持";
    status.textContent = "VibeWbz 目前支持 Windows 和 macOS。";
    progress.hidden = true;
    retry.hidden = false;
  }

  function startDownload() {
    const platform = detectPlatform();
    if (!platform) {
      showUnsupported();
      return;
    }

    const platformName = platform === "windows" ? "Windows" : "macOS";
    title.textContent = `正在下载 ${platformName} 版`;
    status.textContent = "如果下载没有自动开始，请重新打开此链接。";
    progress.hidden = false;
    retry.hidden = true;

    window.setTimeout(() => {
      window.location.replace(downloads[platform]);
    }, 150);
  }

  retry.addEventListener("click", startDownload);
  startDownload();
})();
