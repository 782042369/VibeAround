import { type KeyboardEvent, type ReactNode, type Ref } from "react";
import { DragDropProvider, type DragEndEvent } from "@dnd-kit/react";
import { isSortable } from "@dnd-kit/react/sortable";
import {
  Check,
  ExternalLink,
  FolderOpen,
  Plus,
} from "lucide-react";
import { useI18n } from "@va/i18n";

import { BrandIcon } from "@/components/brand-icon";
import { Button } from "@/components/ui/button";
import { openExternalUrl } from "@/lib/api";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { resolveProfileConnection } from "./connections";
import type {
  LauncherPreferences,
  WorkspaceOption,
} from "./api";
import {
  agentConnectionDef,
  apiTypeProtocolDisplayLabel,
  canDeleteWorkspace,
  connectionAgentId,
  isGlobalDefaultProfile,
  isBridgeAgent,
  isSortableWorkspace,
  profileAvailability,
  profileSummary,
  type ProfileChoice,
} from "./launchModel";
import {
  DefaultBadge,
  DisabledMoreButton,
  DragHandle,
  ProfileActionsMenu,
  BridgeBadge,
  SelectableItemCard,
  SortableItem,
  WorkspaceActionsMenu,
} from "./LaunchBuilderPrimitives";
import type { ConnectionAgentId, ProfileSummary } from "./types";

const CCSWITCH_URL = "https://ccswitch.io/zh";
const GATEWAY_TOKEN_URL = "https://ai.939593.xyz/console/token";
const CCSWITCH_TOKEN_GUIDE_IMAGE =
  "https://lsky.939593.xyz:11111/Olmqcu.png";

function SelectorPanelShell({
  title,
  headerExtra,
  footer,
  children,
}: {
  title: string;
  headerExtra?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="box-border flex max-h-[430px] flex-col overflow-hidden rounded-md border border-border bg-card shadow-sm">
      <div className="flex items-center justify-between gap-3 border-b border-border/70 px-3 py-2">
        <div className="text-[10px] font-semibold uppercase tracking-[0.16em] text-muted-foreground/70">
          {title}
        </div>
        {headerExtra}
      </div>
      {children}
      {footer && (
        <div className="shrink-0 border-t border-border bg-background">
          {footer}
        </div>
      )}
    </section>
  );
}

export function ProfilePanel({
  agentId,
  prefs,
  selected,
  profiles,
  onSelect,
  onSelectApiType,
  onMakeDefault,
  onEditProfile,
  onDuplicateProfile,
  onConnectionSettings,
  onDeleteProfile,
  onReorderProfile,
  onNewProfile,
  busy,
}: {
  agentId: string;
  prefs: LauncherPreferences;
  selected: ProfileChoice | null;
  profiles: ProfileSummary[];
  onSelect: (choice: ProfileChoice) => void;
  onSelectApiType: (profile: ProfileSummary, apiType: string) => void;
  onMakeDefault: (choice: ProfileChoice) => Promise<void>;
  onEditProfile: (profile: ProfileSummary) => void;
  onDuplicateProfile: (profile: ProfileSummary) => void;
  onConnectionSettings: (
    profile: ProfileSummary,
    agentId: ConnectionAgentId,
  ) => void;
  onDeleteProfile: (profile: ProfileSummary) => void;
  onReorderProfile: (fromId: string, toId: string) => void;
  onNewProfile: () => void;
  busy: boolean;
}) {
  const { t } = useI18n();

  function handleProfileDragEnd(event: DragEndEvent) {
    if (event.canceled || busy) return;
    const { source } = event.operation;
    if (!isSortable(source) || source.initialIndex === source.index) return;
    const from = profiles[source.initialIndex]?.id;
    const to = profiles[source.index]?.id;
    if (from && to) onReorderProfile(from, to);
  }

  return (
    <section className="space-y-2">
      <div className="grid gap-3 rounded-md border border-border bg-muted/30 px-3 py-2 md:grid-cols-[minmax(0,1fr)_220px]">
        <div className="min-w-0 space-y-2">
          <p className="text-[11px] leading-4 text-muted-foreground">
            {t(
              "VibeWbz setup launch is built for first-time users. If you have already installed the base environment, ccswitch is recommended.",
            )}
          </p>
          <p className="text-[11px] leading-4 text-muted-foreground">
            {t(
              "If ccswitch is installed, create a gateway token, then use that token in ccswitch.",
            )}
          </p>
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="outline"
              size="xs"
              className="h-7"
              onClick={() => void openExternalUrl(CCSWITCH_URL)}
            >
              <ExternalLink className="h-3 w-3" />
              {t("Open ccswitch")}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="xs"
              className="h-7"
              onClick={() => void openExternalUrl(GATEWAY_TOKEN_URL)}
            >
              <ExternalLink className="h-3 w-3" />
              {t("Create gateway token")}
            </Button>
          </div>
        </div>
        <button
          type="button"
          className="overflow-hidden rounded border border-border bg-background text-left"
          onClick={() => void openExternalUrl(CCSWITCH_TOKEN_GUIDE_IMAGE)}
          title={t("Open guide image")}
        >
          <img
            src={CCSWITCH_TOKEN_GUIDE_IMAGE}
            alt={t("ccswitch gateway token guide")}
            className="h-[96px] w-full object-cover object-top"
            loading="lazy"
          />
        </button>
      </div>
      <div className="grid grid-cols-[repeat(auto-fill,minmax(320px,1fr))] gap-2">
        <SelectableItemCard
          active={false}
          disabled={busy}
          onSelect={onNewProfile}
        >
          <DragHandle
            disabled
            label={t("Configure gateway")}
            disabledReason={t("This item cannot be reordered")}
          />
          <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-dashed border-primary/40 bg-primary/5 text-primary">
            <Plus className="h-6 w-6" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="truncate text-[13px] font-semibold text-primary">
              {t("New profile")}
            </div>
            <div className="truncate text-[11px] text-muted-foreground">
              {t("Add a provider profile")}
            </div>
          </div>
        </SelectableItemCard>

        <DragDropProvider onDragEnd={handleProfileDragEnd}>
          {profiles.map((profile, index) => {
            const availability = profileAvailability(profile, agentId, prefs, t);
            return (
              <SortableItem
                key={profile.id}
                id={profile.id}
                index={index}
                disabled={busy}
              >
                {({ dragHandleRef, isDragging }) => {
                  const summary = profileSummary(profile, agentId, prefs, t);
                  const active =
                    availability.launchable &&
                    selected?.profileId === profile.id;
                  const globalDefaultForProfile = isGlobalDefaultProfile(
                    prefs,
                    agentId,
                    profile.id,
                  );
                  const connectionId = connectionAgentId(agentId);
                  const connection =
                    connectionId
                      ? resolveProfileConnection(
                          profile,
                          prefs.profileConnections,
                          agentConnectionDef(agentId),
                        )
                      : null;
                  const profileApiOptions =
                    connection?.clientApiTypes.filter((client) => client.native) ?? [];
                  const profileApiSelectValue = profileApiOptions.some(
                    (client) => client.apiType === connection?.selectedApiType,
                  )
                    ? connection?.selectedApiType
                    : profileApiOptions[0]?.apiType;
                  return (
                    <SelectableItemCard
                      active={active}
                      disabled={busy || !availability.launchable}
                      isDragging={isDragging}
                      onSelect={() =>
                        onSelect({ kind: "profile", profileId: profile.id })
                      }
                    >
                      <DragHandle
                        label={t("Reorder {{label}}", { label: profile.label })}
                        disabled={busy}
                        disabledReason={
                          busy ? t("Reordering unavailable while launching") : undefined
                        }
                        dragHandleRef={dragHandleRef}
                      />
                      <span className="flex h-12 w-12 shrink-0 items-center justify-center rounded-md border border-border/70 bg-background">
                        <BrandIcon
                          kind="provider"
                          id={profile.provider}
                          label={profile.providerLabel}
                          fallback={profile.providerIcon}
                          framed={false}
                          className="h-9 w-9"
                        />
                      </span>
                      <div className="min-w-0 flex-1">
                        <div className="flex min-w-0 items-center gap-2">
                          <span className="truncate text-[13px] font-semibold">
                            {profile.label}
                          </span>
                          {globalDefaultForProfile && <DefaultBadge />}
                        </div>
                        {summary.bridge && (
                          <div className="mt-0.5">
                            <BridgeBadge label={summary.bridgeLabel} />
                          </div>
                        )}
                        <div
                          className="truncate text-[11px] text-muted-foreground"
                          title={availability.launchable ? summary.route : availability.reason}
                        >
                          {availability.launchable
                            ? summary.route
                            : availability.reason}
                        </div>
                      </div>
                      <div
                        className="flex shrink-0 flex-wrap items-center justify-end gap-2"
                        onClick={(event) => event.stopPropagation()}
                      >
                        {connection &&
                          connection.agent.supportedApiTypes.length > 1 &&
                          profileApiOptions.length > 0 &&
                          profileApiSelectValue && (
                          <Select
                            value={profileApiSelectValue}
                            disabled={busy}
                            onValueChange={(apiType) => onSelectApiType(profile, apiType)}
                          >
                            <SelectTrigger size="sm" className="h-7 w-[clamp(8rem,20vw,160px)] text-[11px]">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              {profileApiOptions.map((option) => (
                                <SelectItem
                                  key={option.apiType}
                                  value={option.apiType}
                                  className="text-xs"
                                >
                                  {apiTypeProtocolDisplayLabel(option.apiType)}
                                </SelectItem>
                              ))}
                            </SelectContent>
                          </Select>
                        )}
                        <ProfileActionsMenu
                          profile={profile}
                          bridgeAvailable={isBridgeAgent(agentId)}
                          disabled={busy}
                          onMakeDefault={
                            globalDefaultForProfile
                              ? undefined
                              : () =>
                                  void onMakeDefault({
                                    kind: "profile",
                                    profileId: profile.id,
                                  })
                          }
                          makeDefaultDisabled={busy || !availability.launchable}
                          onConnectionSettings={(profile) => {
                            const connectionId = connectionAgentId(agentId);
                            if (connectionId) {
                              onConnectionSettings(profile, connectionId);
                            }
                          }}
                          onEditProfile={onEditProfile}
                          onDuplicateProfile={onDuplicateProfile}
                          onDeleteProfile={onDeleteProfile}
                        />
                      </div>
                    </SelectableItemCard>
                  );
                }}
              </SortableItem>
            );
          })}
        </DragDropProvider>
      </div>
    </section>
  );
}

export function WorkspacePanel({
  prefs,
  loading,
  onSelect,
  onDelete,
  onReorder,
  onCreate,
  busy,
}: {
  prefs: LauncherPreferences;
  loading: boolean;
  onSelect: (path: string) => void;
  onDelete: (path: string, label: string) => void;
  onReorder: (fromPath: string, toPath: string) => void;
  onCreate: () => void;
  busy: boolean;
}) {
  const { t } = useI18n();
  const workspaceOptions = [...prefs.workspaceOptions].sort((a, b) => {
    if (a.isDefault === b.isDefault) return 0;
    return a.isDefault ? -1 : 1;
  });
  const sortableWorkspaces = workspaceOptions.filter(isSortableWorkspace);

  function handleWorkspaceDragEnd(event: DragEndEvent) {
    if (event.canceled || busy) return;
    const { source } = event.operation;
    if (!isSortable(source) || source.initialIndex === source.index) return;
    const from = sortableWorkspaces[source.initialIndex]?.path;
    const to = sortableWorkspaces[source.index]?.path;
    if (from && to) onReorder(from, to);
  }

  function handleWorkspaceRowKeyDown(
    event: KeyboardEvent<HTMLDivElement>,
    workspace: WorkspaceOption,
  ) {
    if (busy || (event.key !== "Enter" && event.key !== " ")) return;
    event.preventDefault();
    onSelect(workspace.path);
  }

  function renderWorkspaceRow(
    workspace: WorkspaceOption,
    dragHandleRef?: Ref<HTMLSpanElement>,
    isDragging = false,
  ) {
    const active = workspace.path === prefs.workspace;
    const sortable = isSortableWorkspace(workspace);
    const canDelete = canDeleteWorkspace(workspace);
    return (
      <div
        role="button"
        key={workspace.path}
        tabIndex={busy ? -1 : 0}
        className={`group flex w-full items-center gap-2 px-3 py-2 text-left transition-colors ${
          active
            ? "bg-primary/10 text-primary"
            : "text-foreground hover:bg-accent/50"
        } ${busy ? "cursor-not-allowed opacity-60" : "cursor-pointer"} ${
          isDragging ? "opacity-55" : ""
        }`}
        aria-disabled={busy}
        data-dragging={isDragging ? "true" : undefined}
        onClick={() => {
          if (!busy) onSelect(workspace.path);
        }}
        onKeyDown={(event) => handleWorkspaceRowKeyDown(event, workspace)}
      >
        <DragHandle
          disabled={!sortable || busy}
          label={t("Reorder {{label}}", { label: workspace.label })}
          disabledReason={
            !sortable
              ? workspace.isDefault
                ? t("Default workspace is fixed")
                : t("This item cannot be reordered")
              : t("Reordering unavailable while launching")
          }
          dragHandleRef={dragHandleRef}
        />
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-background text-muted-foreground">
          <FolderOpen className="h-4 w-4" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex min-w-0 flex-wrap items-center gap-2">
            <span className="truncate text-[13px] font-semibold">
              {workspace.label}
            </span>
            {workspace.isDefault && <DefaultBadge />}
          </span>
          <Tooltip>
            <TooltipTrigger asChild>
              <span className="block truncate text-[11px] text-muted-foreground">
                {workspace.detail}
              </span>
            </TooltipTrigger>
            <TooltipContent className="max-w-[min(520px,calc(100vw-2rem))] break-all text-left">
              {workspace.path}
            </TooltipContent>
          </Tooltip>
        </span>
        {active ? (
          <Check className="h-4 w-4 shrink-0 text-primary" />
        ) : (
          <span className="h-4 w-4 shrink-0" aria-hidden="true" />
        )}
        <span
          className="flex shrink-0 flex-wrap items-center justify-end gap-2"
          onClick={(event) => event.stopPropagation()}
        >
          {canDelete ? (
            <WorkspaceActionsMenu
              workspace={workspace}
              onDelete={(target) => onDelete(target.path, target.label)}
            />
          ) : (
            <DisabledMoreButton
              reason={
                workspace.isDefault
                  ? t("Default workspace cannot be edited or deleted")
                  : t("No actions available")
              }
            />
          )}
        </span>
      </div>
    );
  }

  return (
    <SelectorPanelShell
      title={t("Switch workspace")}
      footer={
        <button
          type="button"
          disabled={busy}
          className="flex w-full cursor-pointer items-center gap-2 px-3 py-2 text-left text-primary transition-colors hover:bg-primary/5 disabled:cursor-not-allowed disabled:opacity-60"
          onClick={onCreate}
        >
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-dashed border-primary/40 bg-primary/5">
            <Plus className="h-4 w-4" />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[13px] font-semibold">
              {t("New workspace...")}
            </span>
            <span className="block truncate text-[11px] text-muted-foreground">
              {t("Choose folder...")}
            </span>
          </span>
        </button>
      }
    >
      {loading && workspaceOptions.length === 0 && (
        <p className="px-3 py-2 text-xs text-muted-foreground">
          {t("Loading…")}
        </p>
      )}
      <DragDropProvider onDragEnd={handleWorkspaceDragEnd}>
        <div className="min-h-0 flex-1 divide-y divide-border/60 overflow-y-auto">
          {workspaceOptions.map((workspace) => {
            if (!isSortableWorkspace(workspace)) {
              return renderWorkspaceRow(workspace);
            }
            const index = sortableWorkspaces.findIndex(
              (sortable) => sortable.path === workspace.path,
            );
            return (
              <SortableItem
                key={workspace.path}
                id={workspace.path}
                index={index}
                disabled={busy}
              >
                {({ dragHandleRef, isDragging }) =>
                  renderWorkspaceRow(workspace, dragHandleRef, isDragging)
                }
              </SortableItem>
            );
          })}
        </div>
      </DragDropProvider>
    </SelectorPanelShell>
  );
}
