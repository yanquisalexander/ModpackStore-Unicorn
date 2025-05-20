
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { LucideAlertOctagon, LucideClipboard, LucideEye } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";

interface CrashDialogProps {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    exitCode: number;
    errorMessage: string;
    data: any;
    onViewCrashReport?: () => void;
    onSendCrashReport?: () => void;
}

export const InstanceCrashDialog = ({
    open,
    onOpenChange,
    exitCode,
    errorMessage,
    data,
    onViewCrashReport,
    onSendCrashReport,
}: CrashDialogProps) => {
    const [copied, setCopied] = useState(false);

    console.log(data)
    useEffect(() => {
        if (copied) {
            const timeout = setTimeout(() => setCopied(false), 2000);
            return () => clearTimeout(timeout);
        }
    }, [copied]);

    const copyExitCode = () => {
        navigator.clipboard.writeText(exitCode.toString());
        setCopied(true);
        toast.success("Código de error copiado al portapapeles");
    };

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className="bg-neutral-900 text-white border-none max-w-md">
                <DialogHeader className="text-center">

                    <DialogTitle className="text-sm font-normal text-center">ERROR</DialogTitle>
                    <DialogTitle className="text-2xl font-semibold text-center mb-4">Game crashed</DialogTitle>
                    <DialogDescription className="text-white text-center">
                        An unexpected issue occurred and the game has crashed.
                        We're sorry for
                        the inconvenience.
                        <br />
                        {
                            data.detectedError && (
                                <code className="text-red-500 text-sm">
                                    {data.detectedError}
                                </code>
                            )
                        }

                        {
                            data.stderr && (
                                <code className="block bg-neutral-800 text-sm p-2 rounded mt-2">
                                    {data.stderr}
                                </code>
                            )
                        }

                    </DialogDescription>
                </DialogHeader>

                <div className="text-center text-white text-sm mb-4">
                    {
                        errorMessage
                            ? errorMessage
                            : "We're sorry, but we couldn't retrieve the error message. Please check the logs for more details."
                    }

                </div>

                {/* <div className="flex justify-center mb-4">
                    <Button
                        variant="link"
                        className="text-blue-400 hover:text-blue-300 flex items-center gap-1"
                        onClick={() => window.open('https://support.example.com/crash-info', '_blank')}
                    >
                        Click here for more information.
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                            <path d="M7 17L17 7" />
                            <path d="M7 7h10v10" />
                        </svg>
                    </Button>
                </div>

                <div className="flex items-center justify-center gap-2 mb-6">
                    <span className="text-white">Exit Code: {exitCode}</span>
                    <button onClick={copyExitCode} className="text-gray-400 hover:text-white transition-colors">
                        {copied ? (
                            <LucideClipboard className="size-4 text-green-500" />
                        ) : (
                            <LucideClipboard className="size-4" />
                        )}
                    </button>
                </div>

                <div className="flex gap-2 justify-center">
                    <Button
                        variant="outline"
                        className="bg-neutral-800 hover:bg-neutral-700 text-white border-neutral-700"
                        onClick={onViewCrashReport}
                    >
                        <LucideEye className="size-4 mr-2" />
                        View crash report
                    </Button>
                    <Button
                        className="bg-green-600 hover:bg-green-700 text-white"
                        onClick={onSendCrashReport}
                    >
                        Send crash report
                    </Button>
                </div> */}
            </DialogContent>
        </Dialog>
    );
};

