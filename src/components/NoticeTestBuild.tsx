import { useState } from 'react';
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { LucideTestTube2 } from 'lucide-react';

const NoticeTestBuild = () => {
    const [_open, setOpen] = useState(true);

    return (
        <AlertDialog defaultOpen={true} onOpenChange={setOpen}>
            <AlertDialogContent className="max-w-md">
                <AlertDialogHeader>
                    <div className="flex items-center gap-2 text-amber-600">
                        <LucideTestTube2 size={20} />
                        <AlertDialogTitle>Versión de Desarrollo</AlertDialogTitle>
                    </div>
                    <AlertDialogDescription className="pt-2">
                        <span className="mb-2 text-base text-foreground">
                            Estás utilizando una <span className="font-semibold">versión en desarrollo</span> de Modpack Store.
                        </span>
                        <span className="text-sm text-foreground">
                            Esta versión puede contener errores y funcionalidades incompletas.
                        </span>
                    </AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogAction className="bg-amber-600 hover:bg-amber-700">
                        Entendido
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    );
};

export default NoticeTestBuild;