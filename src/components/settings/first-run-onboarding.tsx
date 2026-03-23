import { Check, ChevronRight, Shield, Sparkles } from "lucide-react";
import { Badge } from "#/components/ui/badge.tsx";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";

type PermissionStep = {
  id: string;
  title: string;
  description: string;
  granted: boolean;
  busy: boolean;
  buttonLabel: string;
  onClick: () => void;
};

export function FirstRunOnboarding({
  canFinish,
  isMacOS,
  onContinue,
  onSkip,
  permissionSteps,
}: {
  canFinish: boolean;
  isMacOS: boolean;
  onContinue: () => void;
  onSkip: () => void;
  permissionSteps: PermissionStep[];
}) {
  return (
    <div className="min-h-full bg-[radial-gradient(circle_at_top_left,_rgba(22,163,74,0.18),_transparent_38%),linear-gradient(180deg,_rgba(248,250,252,0.98),_rgba(255,255,255,1))] px-5 py-5 text-sm text-foreground">
      <div className="mx-auto flex max-w-[760px] flex-col gap-4">
        <Card className="overflow-hidden border-emerald-200/80 bg-white/90 shadow-lg shadow-emerald-950/5">
          <CardHeader className="gap-4 border-b border-emerald-100/80 bg-[linear-gradient(135deg,_rgba(240,253,244,0.96),_rgba(255,255,255,0.92))] pb-5">
            <div className="flex flex-wrap items-center gap-2">
              <Badge className="gap-1 border-0 bg-emerald-600/10 px-2.5 py-1 text-emerald-900 hover:bg-emerald-600/10">
                <Sparkles className="size-3.5" />
                First launch
              </Badge>
              <Badge variant="outline">Visible setup flow</Badge>
            </div>
            <div className="space-y-2">
              <CardTitle className="text-[26px] leading-tight">
                Chord needs two macOS permissions before it can listen and click.
              </CardTitle>
              <CardDescription className="max-w-[620px] text-[15px] leading-6 text-slate-600">
                This setup window stays visible on first launch so the app does not disappear into a hidden tray-only state.
                Grant Accessibility and Input Monitoring, then continue into the full settings view.
              </CardDescription>
            </div>
          </CardHeader>
          <CardContent className="grid gap-4 px-5 py-5">
            {permissionSteps.map((step, index) => (
              <div
                key={step.id}
                className="rounded-2xl border border-slate-200/90 bg-slate-50/75 p-4 shadow-sm shadow-slate-950/5"
              >
                <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                  <div className="space-y-1.5">
                    <div className="flex items-center gap-2">
                      <div className="flex size-7 items-center justify-center rounded-full bg-slate-900 text-xs font-semibold text-white">
                        {index + 1}
                      </div>
                      <p className="font-semibold text-slate-950">{step.title}</p>
                      {step.granted ? (
                        <Badge className="border-0 bg-emerald-600/10 text-emerald-800 hover:bg-emerald-600/10">
                          <Check className="mr-1 size-3.5" />
                          Granted
                        </Badge>
                      ) : (
                        <Badge variant="outline">Required</Badge>
                      )}
                    </div>
                    <p className="max-w-[520px] text-sm leading-6 text-slate-600">{step.description}</p>
                  </div>

                  <Button
                    type="button"
                    variant={step.granted ? "outline" : "default"}
                    className={step.granted ? "" : "bg-slate-950 text-white hover:bg-slate-800"}
                    onClick={step.onClick}
                    disabled={step.busy}
                  >
                    {step.busy ? "Opening..." : step.buttonLabel}
                  </Button>
                </div>
              </div>
            ))}

            <div className="rounded-2xl border border-amber-200/80 bg-amber-50/90 p-4 text-sm leading-6 text-amber-950">
              <div className="flex items-center gap-2 font-medium">
                <Shield className="size-4" />
                Why both permissions are needed
              </div>
              <p className="mt-2">
                Accessibility lets Chord execute UI automation. Input Monitoring lets it detect the global trigger. macOS may
                ask you to reopen the app after enabling them.
              </p>
            </div>

            <div className="flex flex-col-reverse gap-3 border-t border-slate-200 pt-4 sm:flex-row sm:items-center sm:justify-between">
              <Button type="button" variant="ghost" onClick={onSkip}>
                Finish Later
              </Button>
              <div className="flex items-center gap-2 self-end sm:self-auto">
                {isMacOS && !canFinish ? (
                  <p className="text-xs text-slate-500">Grant both permissions to finish setup now.</p>
                ) : null}
                <Button
                  type="button"
                  onClick={onContinue}
                  disabled={isMacOS && !canFinish}
                  className="bg-emerald-600 text-white hover:bg-emerald-700"
                >
                  Continue
                  <ChevronRight className="size-4" />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
