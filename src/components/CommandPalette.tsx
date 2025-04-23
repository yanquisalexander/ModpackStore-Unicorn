import { useState, useEffect, useRef } from 'react';
import {
    LucideSearch,
    LucideSettings,
    LucideUser,
    LucideFolder,
    LucideDownload,
    LucidePackage,
    LucideServer,
    LucideCode,
    LucideChevronRight,
    LucideGlobe,
    LucidePlay,
    LucideEdit,
    LucideTrash,
    LucideShield,
    LucideUsers,
    LucideRefreshCw,
    LucideHardDrive,
    LucideBox,
    LucideLoader,
    LucideAppWindowMac
} from 'lucide-react';
import { Dialog, DialogContent } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { invoke } from '@tauri-apps/api/core';
import { TauriCommandReturns } from "@/types/TauriCommandReturns";
import { navigate } from "wouter/use-browser-location";
import { useReloadApp } from "@/stores/ReloadContext";

export default function ModpackCommandPalette() {
    const [isOpen, setIsOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [activeIndex, setActiveIndex] = useState(0);
    const [isLoading, setIsLoading] = useState(false);
    const [instanceResults, setInstanceResults] = useState<TauriCommandReturns['search_instances']>([]);
    const [searchTimeout, setSearchTimeout] = useState(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const { showReloadDialog } = useReloadApp(); // Usar el hook para acceder a la funcionalidad de recarga



    // Static commands
    const staticCommandGroups = [
        {
            name: 'Acciones Rápidas',
            commands: [
                { id: 'browse-modpacks', label: 'Explorar Modpacks', icon: <LucideGlobe size={16} /> },
                { id: 'my-instances', label: 'Mis Instancias', icon: <LucideFolder size={16} /> },
                { id: 'check-updates', label: 'Buscar Actualizaciones', icon: <LucideRefreshCw size={16} />, shortcut: '' }
            ]
        },

        {
            name: 'Configuración',
            commands: [
                { id: 'settings-general', label: 'Configuración General', icon: <LucideSettings size={16} />, shortcut: 'Ctrl + ,' },
            ]
        },
        {
            name: 'Avanzado',
            commands: [
                { id: 'restart-app', label: 'Recargar aplicación', icon: <LucideAppWindowMac size={16} /> },
            ]
        }
    ];

    // Create a dynamic command group from search results
    const getDynamicCommandGroups = () => {
        if (instanceResults.length === 0) {
            return [];
        }

        return [
            {
                name: 'Instancias Encontradas',
                commands: [
                    ...instanceResults.map(instance => ({
                        id: `launch-${instance.instanceId}`,
                        label: instance.instanceName,
                        icon: <LucidePlay size={16} />,
                        meta: `v${instance.minecraftVersion}`,
                        instanceIcon: instance.iconUrl || '📦'
                    })),

                ]
            }
        ];
    };

    // Combine static and dynamic command groups
    const commandGroups = [...staticCommandGroups, ...getDynamicCommandGroups()];

    // Search instances using Tauri command
    const searchInstances = async (query: string) => {
        try {
            setIsLoading(true);
            // Invoke the Tauri command for searching instances
            const results = await invoke<TauriCommandReturns['search_instances']>('search_instances', { query });
            setInstanceResults(results || []);
            setIsLoading(false);
        } catch (error) {
            console.error('Error searching instances:', error);
            setInstanceResults([]);
            setIsLoading(false);
        }
    };

    // Trigger search when query changes (with debounce)
    useEffect(() => {
        if (isOpen) {
            // Clear any existing timeout
            if (searchTimeout) {
                clearTimeout(searchTimeout);
            }

            // Set a new timeout to avoid excessive API calls
            const timeout = setTimeout(() => {
                searchInstances(searchQuery);
            }, 300);

            // @ts-ignore
            setSearchTimeout(timeout);
        }

        return () => {
            if (searchTimeout) {
                clearTimeout(searchTimeout);
            }
        };
    }, [searchQuery, isOpen]);

    // Reset and search when opening the palette
    useEffect(() => {
        if (isOpen) {
            setSearchQuery('');
            setActiveIndex(0);
            searchInstances('');
        }
    }, [isOpen]);

    // Filter commands based on search query
    const filteredCommands = searchQuery.trim() === ''
        ? commandGroups
        : commandGroups
            .map(group => ({
                ...group,
                commands: group.commands.filter(command =>
                    command.label.toLowerCase().includes(searchQuery.toLowerCase()) ||
                    ('meta' in command && command.meta.toLowerCase().includes(searchQuery.toLowerCase()))
                )
            }))
            .filter(group => group.commands.length > 0);

    // Flatten commands for keyboard navigation
    const flattenedCommands = filteredCommands.flatMap(group => group.commands);

    // Handle keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Open command palette with Ctrl+K
            if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
                e.preventDefault();
                setIsOpen(true);
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, []);

    // Handle keyboard navigation within the command palette
    useEffect(() => {
        if (!isOpen) return;

        const handleKeyboardNavigation = (e: KeyboardEvent) => {
            switch (e.key) {
                case 'ArrowDown':
                    e.preventDefault();
                    setActiveIndex(prevIndex =>
                        prevIndex < flattenedCommands.length - 1 ? prevIndex + 1 : prevIndex
                    );
                    break;
                case 'ArrowUp':
                    e.preventDefault();
                    setActiveIndex(prevIndex =>
                        prevIndex > 0 ? prevIndex - 1 : prevIndex
                    );
                    break;
                case 'Enter':
                    e.preventDefault();
                    if (flattenedCommands.length > 0) {
                        executeCommand(flattenedCommands[activeIndex].id);
                    }
                    break;
                case 'Escape':
                    e.preventDefault();
                    setIsOpen(false);
                    break;
                default:
                    break;
            }
        };

        window.addEventListener('keydown', handleKeyboardNavigation);
        return () => window.removeEventListener('keydown', handleKeyboardNavigation);
    }, [isOpen, flattenedCommands, activeIndex]);

    // Focus input when dialog opens
    useEffect(() => {
        if (isOpen && inputRef.current) {
            setTimeout(() => {
                inputRef.current?.focus();
            }, 100);
        }
    }, [isOpen]);

    // Execute command and close palette
    const executeCommand = (commandId: string) => {
        console.log(`Executing command: ${commandId}`);
        if (commandId.startsWith('launch-')) {
            const instanceId = commandId.split('-')[1];
            navigate(`/prelaunch/${instanceId}`);
        }

        switch (commandId) {
            case 'my-instances':
                navigate('/my-instances');
                break;
            case 'browse-modpacks':
                navigate('/');
                break;
            case 'restart-app':
                showReloadDialog(); // Show the reload dialog
                break;
        }
        setIsOpen(false);
    };

    return (
        <Dialog open={isOpen} onOpenChange={setIsOpen}>
            <DialogContent className="sm:max-w-xl p-0 gap-0 overflow-hidden">
                <div className="flex items-center border-b p-4">
                    {isLoading ? (
                        <LucideLoader className="mr-2 h-5 w-5 shrink-0 text-gray-400 animate-spin" />
                    ) : (
                        <LucideSearch className="mr-2 h-5 w-5 shrink-0 text-gray-400" />
                    )}
                    <Input
                        ref={inputRef}
                        placeholder="Buscar instancias, comandos, acciones..."
                        value={searchQuery}
                        onChange={(e) => {
                            setSearchQuery(e.target.value);
                            setActiveIndex(0);
                        }}
                        className="border-0 focus-visible:ring-0 focus-visible:ring-offset-0 pl-0"
                    />
                    <div className="flex items-center ml-auto gap-1">
                        <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-gray-50 px-1.5 text-xs font-medium text-gray-500">
                            <span className="text-xs">
                                {navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}
                            </span>K
                        </kbd>
                    </div>
                </div>
                <ScrollArea className="max-h-96 overflow-y-auto">
                    {isLoading && filteredCommands.length === 0 ? (
                        <div className="py-12 text-center">
                            <LucideLoader className="h-8 w-8 animate-spin mx-auto text-gray-400" />
                            <p className="mt-2 text-sm text-gray-500">Buscando instancias...</p>
                        </div>
                    ) : filteredCommands.length === 0 ? (
                        <div className="py-6 text-center text-gray-500">
                            No se encontraron resultados para "{searchQuery}"
                        </div>
                    ) : (
                        <div className="py-2">
                            {filteredCommands.map((group, groupIndex) => (
                                <div key={groupIndex} className="px-2 mb-2">
                                    <div className="px-2 py-1.5 text-xs font-semibold text-gray-500">
                                        {group.name}
                                    </div>
                                    {group.commands.map((command) => {
                                        // Calculate the absolute index in the flattened list
                                        const absoluteIndex = flattenedCommands.findIndex(cmd => cmd.id === command.id);
                                        const isActive = absoluteIndex === activeIndex;

                                        return (
                                            <div
                                                key={command.id}
                                                onClick={() => executeCommand(command.id)}
                                                onMouseEnter={() => setActiveIndex(absoluteIndex)}
                                                className={`
                          flex items-center justify-between rounded-md px-2 py-1.5 text-sm cursor-pointer
                          ${isActive ? 'bg-blue-500/20 text-blue-100' : 'hover:bg-gray-500/10'}
                        `}
                                            >
                                                <div className="flex items-center gap-2">
                                                    {command.icon ? (
                                                        <span className="flex items-center justify-center w-6 h-6 rounded-md bg-gray-500/10">
                                                            {command.icon}
                                                        </span>
                                                    ) : (
                                                        <span className="flex items-center justify-center w-6 h-6 rounded-md bg-gray-100">
                                                            {command.icon}
                                                        </span>
                                                    )}
                                                    <span>{command.label}</span>
                                                    {'meta' in command && command.meta && (
                                                        <Badge variant="outline" className="ml-1 text-xs">
                                                            {command.meta}
                                                        </Badge>
                                                    )}
                                                </div>
                                                <div className="flex items-center gap-2">
                                                    {'shortcut' in command && command.shortcut && (
                                                        <Badge variant="outline" className="text-xs">
                                                            {command.shortcut}
                                                        </Badge>
                                                    )}
                                                    {isActive && <LucideChevronRight size={14} />}
                                                </div>
                                            </div>
                                        );
                                    })}
                                </div>
                            ))}
                        </div>
                    )}
                </ScrollArea>
                <div className="border-t px-4 py-2 text-xs text-gray-500">
                    <div className="flex items-center justify-between">
                        <div>
                            <span className="mr-2">↑↓</span> para navegar
                            <span className="mx-2">↵</span> para seleccionar
                            <span className="mx-2">esc</span> para cerrar
                        </div>
                        <div>
                            <Badge variant="outline" className="text-xs">
                                {flattenedCommands.length} resultados
                            </Badge>
                        </div>
                    </div>
                </div>
            </DialogContent>
        </Dialog>
    );
}