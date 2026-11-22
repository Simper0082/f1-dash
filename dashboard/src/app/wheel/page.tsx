"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

import { useDataEngine } from "@/hooks/useDataEngine";
import { useStores } from "@/hooks/useStores";
import { useSocket } from "@/hooks/useSocket";

import { useDataStore } from "@/stores/useDataStore";
import { useSettingsStore } from "@/stores/useSettingsStore";

import { getTrackStatusMessage } from "@/lib/getTrackStatusMessage";
import { convertTemperature } from "@/lib/temperature";
import { env } from "@/env";

// Compact 480 x 320 landscape screen for a steering wheel display (e.g. a 3.5" ESP32-S3 board).
// Usage: /wheel?driver=3   (driver = racing number, defaults to 3)

const PANEL = "#17171b";
const MUTED = "#9a9aa4";

const TYRE_COLORS: Record<string, { ring: string; letter: string }> = {
	SOFT: { ring: "#ef4444", letter: "S" },
	MEDIUM: { ring: "#facc15", letter: "M" },
	HARD: { ring: "#f2f2f2", letter: "H" },
	INTERMEDIATE: { ring: "#2fb34a", letter: "I" },
	WET: { ring: "#3b82f6", letter: "W" },
};


// Extra live info from the realtime service (/api/wheel), polled once a second.
type WheelExtra = {
	track?: { kind: string; age_s?: number | null; reason?: { text: string; age_s?: number | null } | null };
	pit?: { in_pit?: boolean | null; in_for_s?: number | null; last_lane_s?: number | null; last_box_s_est?: number | null; last_out_ago_s?: number | null };
	penalty?: { type: string; reason: string; served: boolean; age_s?: number | null } | null;
};

const SC_BANNER_SECONDS = 45;
const REASON_BANNER_SECONDS = 20;
const PIT_EXIT_BANNER_SECONDS = 8;
const PENALTY_BANNER_SECONDS = 20;

type BannerSpec = { key: string; bg: string; fg: string; title: string; detail?: string; flash?: boolean };

function useWheelExtra(driverNr: string): WheelExtra | null {
	const [extra, setExtra] = useState<WheelExtra | null>(null);
	useEffect(() => {
		let stop = false;
		const tick = async () => {
			try {
				const res = await fetch(`${env.NEXT_PUBLIC_LIVE_URL}/api/wheel?driver=${driverNr}`);
				if (!stop && res.ok) setExtra(await res.json());
			} catch {
				// keep the last value, the next poll will retry
			}
		};
		tick();
		const id = setInterval(tick, 1000);
		return () => {
			stop = true;
			clearInterval(id);
		};
	}, [driverNr]);
	return extra;
}

function buildBanners(extra: WheelExtra | null): BannerSpec[] {
	if (!extra) return [];
	const out: BannerSpec[] = [];
	const pen = extra.penalty;
	if (pen && !pen.served && (pen.age_s ?? 0) < PENALTY_BANNER_SECONDS) {
		out.push({ key: "pen", bg: "#dc2626", fg: "#fff", title: pen.type.replace("SECOND", "SEC"), detail: pen.reason, flash: true });
	}
	const pit = extra.pit;
	if (pit?.in_pit) {
		out.push({ key: "pit", bg: "#facc15", fg: "#111", title: "IN PIT", detail: pit.in_for_s != null ? `${pit.in_for_s.toFixed(1)} s` : undefined });
	} else if (pit?.last_lane_s != null && (pit.last_out_ago_s ?? 99) < PIT_EXIT_BANNER_SECONDS) {
		out.push({
			key: "exit",
			bg: "#facc15",
			fg: "#111",
			title: "PIT EXIT",
			detail: `LANE ${pit.last_lane_s.toFixed(1)} s · BOX ~${(pit.last_box_s_est ?? 3).toFixed(0)} s est.`,
		});
	}
	const t = extra.track;
	if (t && t.kind !== "clear" && (t.age_s ?? 0) < SC_BANNER_SECONDS) {
		const label = { sc: "SAFETY CAR DEPLOYED", vsc: "VIRTUAL SAFETY CAR", yellow: "YELLOW FLAG", red: "RED FLAG" }[t.kind] ?? t.kind.toUpperCase();
		out.push({ key: "track", bg: t.kind === "red" ? "#dc2626" : "#facc15", fg: t.kind === "red" ? "#fff" : "#111", title: label, flash: t.kind === "sc" || t.kind === "vsc" });
	}
	// the reason usually arrives well after the deployment, so it has its own window
	if (t && t.kind !== "clear" && t.reason && (t.reason.age_s ?? 0) < REASON_BANNER_SECONDS) {
		out.push({ key: "reason", bg: "#3f3f46", fg: "#f2f2f2", title: "REASON", detail: t.reason.text });
	}
	return out.slice(0, 2);
}

const lapToSeconds = (value: string | undefined): number | null => {
	if (!value) return null;
	const m = value.match(/^(?:(\d+):)?(\d+(?:\.\d+)?)$/);
	if (!m) return null;
	return (m[1] ? parseInt(m[1], 10) * 60 : 0) + parseFloat(m[2]);
};

export default function WheelPage() {
	return (
		<Suspense fallback={null}>
			<Wheel />
		</Suspense>
	);
}

function Wheel() {
	const driverNr = useSearchParams().get("driver") ?? "3";
	const extra = useWheelExtra(driverNr);
	const banners = buildBanners(extra);

	const stores = useStores();
	const { handleInitial, handleUpdate } = useDataEngine(stores);
	const { connected } = useSocket({ handleInitial, handleUpdate });

	const state = useDataStore((s) => s.state);
	const temperatureUnit = useSettingsStore((s) => s.temperatureUnit);

	const driver = state?.DriverList?.[driverNr];
	const timing = state?.TimingData?.Lines?.[driverNr];
	const stints = state?.TimingAppData?.Lines?.[driverNr]?.Stints;
	const stint = stints && stints.length > 0 ? stints[stints.length - 1] : undefined;
	const tyre = stint?.Compound ? TYRE_COLORS[stint.Compound] : undefined;

	const lines = state?.TimingData?.Lines ?? {};
	const cars = Object.values(lines).filter((l) => l && l.Position).length;

	const leaderBest = Object.values(lines).find((l) => l?.Position === "1")?.BestLapTime?.Value;
	const myBest = timing?.BestLapTime?.Value;
	const bestDelta =
		lapToSeconds(myBest) !== null && lapToSeconds(leaderBest) !== null
			? (lapToSeconds(myBest) as number) - (lapToSeconds(leaderBest) as number)
			: null;
	// The feed gives the leader "LAP n" here, which is not a gap: treat it as no car ahead.
	const rawInterval = timing?.IntervalToPositionAhead?.Value;
	const interval = rawInterval && !String(rawInterval).startsWith("LAP") ? rawInterval : undefined;

	const track = getTrackStatusMessage(state?.TrackStatus ? parseInt(state.TrackStatus.Status) : undefined);
	const weather = state?.WeatherData;
	const unitLabel = temperatureUnit === "fahrenheit" ? "°F" : "°C";
	const lapCount = state?.LapCount;
	const teamColour = driver?.TeamColour ? `#${driver.TeamColour}` : "#4b5563";

	return (
		<div className="flex min-h-screen items-center justify-center bg-black">
			<div
				className="relative overflow-hidden rounded-md"
				style={{ width: 480, height: 320, background: "#0a0a0c", color: "#f2f2f2" }}
			>
				<div
					className="flex items-center justify-between px-3 text-xs"
					style={{ height: 30, background: PANEL, color: "#b8b8c0" }}
				>
					<span
						className="rounded px-2 py-0.5 font-medium"
						style={{ background: track?.hex ?? "#3f3f46", color: "#111" }}
					>
						{track ? track.message.toUpperCase() : connected ? "WAITING" : "OFFLINE"}
					</span>
					<span>
						{state?.SessionInfo
							? `${state.SessionInfo.Name.toUpperCase()} · ${state.SessionInfo.Meeting.Location.toUpperCase()}`
							: "NO SESSION"}
					</span>
					<span>
						{weather
							? `AIR ${convertTemperature(parseFloat(weather.AirTemp), temperatureUnit)}${unitLabel} · TRACK ${convertTemperature(parseFloat(weather.TrackTemp), temperatureUnit)}${unitLabel}`
							: ""}
					</span>
				</div>

				<style>{`@keyframes wheelflash{0%,49%{opacity:1}50%,100%{opacity:.35}}`}</style>
				<div className="absolute flex flex-col gap-1" style={{ left: 12, right: 12, top: "50%", transform: "translateY(-50%)", zIndex: 10 }}>
					{banners.map((b) => (
						<div
							key={b.key}
							className="flex flex-col items-center justify-center rounded-lg px-3 text-center font-semibold shadow-lg"
							style={{
								minHeight: 72,
								background: b.bg,
								color: b.fg,
								animation: b.flash ? "wheelflash 1s steps(1) infinite" : undefined,
							}}
						>
							<span className="whitespace-nowrap text-[28px] leading-tight">{b.title}</span>
							{b.detail ? <span className="line-clamp-2 max-w-full text-[15px] font-medium leading-tight">{b.detail}</span> : null}
						</div>
					))}
				</div>

				<div className="absolute left-0 w-1" style={{ top: 30, height: 290, background: teamColour }} />

				<div className="absolute" style={{ left: 20, top: 40, width: 240 }}>
					<div className="text-[13px] tracking-wider" style={{ color: MUTED }}>
						{driver ? `${driver.Tla} · ${driver.RacingNumber}` : `# ${driverNr}`}
					</div>
					<div className="flex items-baseline gap-1.5 leading-none">
						<span className="text-3xl font-medium" style={{ color: MUTED }}>
							P
						</span>
						<span className="font-medium" style={{ fontSize: 132, letterSpacing: -4 }}>
							{timing?.Position ?? "-"}
						</span>
					</div>
					<div className="mt-0.5 text-[13px]" style={{ color: MUTED }}>
						{cars > 0 ? `of ${cars} cars` : ""}
					</div>
				</div>

				<div className="absolute flex flex-col gap-2.5" style={{ right: 14, top: 40, width: 190 }}>
					<Panel>
						<span className="text-xs" style={{ color: MUTED }}>
							LAP
						</span>
						<span className="text-3xl font-medium">
							{timing?.NumberOfLaps ?? "-"}
							{lapCount && lapCount.TotalLaps > 0 ? (
								<span className="text-sm" style={{ color: MUTED }}>
									{" "}
									/ {lapCount.TotalLaps}
								</span>
							) : null}
						</span>
					</Panel>

					<div className="flex items-center gap-2.5 rounded-lg px-3 py-2" style={{ background: PANEL }}>
						<span
							className="flex items-center justify-center rounded-full text-[15px] font-medium"
							style={{
								width: 34,
								height: 34,
								border: `4px solid ${tyre?.ring ?? "#52525b"}`,
							}}
						>
							{tyre?.letter ?? "?"}
						</span>
						<div>
							<div className="text-sm font-medium">{stint?.Compound ? stint.Compound.slice(0, 5) : "TYRE"}</div>
							<div className="text-xs" style={{ color: MUTED }}>
								{stint?.TotalLaps !== undefined ? `${stint.TotalLaps} laps old` : ""}
							</div>
						</div>
					</div>

					<Panel>
						<span className="text-xs" style={{ color: MUTED }}>
							PIT STOPS
						</span>
						<span className="text-2xl font-medium">{stints ? Math.max(stints.length - 1, 0) : "-"}</span>
					</Panel>
				</div>

				<div
					className="absolute grid grid-cols-3 items-center rounded-lg text-center"
					style={{ left: 12, right: 12, bottom: 10, height: 74, background: PANEL }}
				>
					<Cell label="LAST" value={timing?.LastLapTime?.Value || "-"} sub={timing?.InPit ? "in pit" : ""} />
					<Cell label="BEST" value={myBest || "-"} sub="" />
					{interval ? (
						<Cell label="TO CAR AHEAD" value={interval} sub="" />
					) : (
						<Cell
							label="TO P1 BEST"
							value={bestDelta === null ? "-" : `${bestDelta >= 0 ? "+" : "-"}${Math.abs(bestDelta).toFixed(3)}`}
							sub={leaderBest ?? ""}
							valueColor={bestDelta !== null && bestDelta > 0 ? "#ff6b6b" : undefined}
						/>
					)}
				</div>
			</div>
		</div>
	);
}

function Panel({ children }: { children: React.ReactNode }) {
	return (
		<div className="flex items-center justify-between rounded-lg px-3 py-2" style={{ background: PANEL }}>
			{children}
		</div>
	);
}

function Cell({ label, value, sub, valueColor }: { label: string; value: string; sub: string; valueColor?: string }) {
	return (
		<div>
			<div className="text-[11px]" style={{ color: MUTED }}>
				{label}
			</div>
			<div className="text-[22px] font-medium" style={{ color: valueColor }}>
				{value}
			</div>
			<div className="text-[11px]" style={{ color: MUTED }}>
				{sub}
			</div>
		</div>
	);
}
