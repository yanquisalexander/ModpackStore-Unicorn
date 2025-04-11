import { useState, useRef } from "react"
import { ModpackCard } from "./ModpackCard"
import { ChevronLeft, ChevronRight } from "lucide-react"

export const CategoryHorizontalSection = ({
    title,
    modpacks = [],
    href = "/prelaunch/",
    viewAllLink = "#"
}: {
    title: string,
    modpacks: any[],
    href?: string,
    viewAllLink?: string
}) => {
    const scrollContainerRef = useRef<HTMLDivElement>(null)
    const [showLeftArrow, setShowLeftArrow] = useState(false)
    const [showRightArrow, setShowRightArrow] = useState(true)

    // Handle scroll to check if arrows should be shown
    const handleScroll = () => {
        if (!scrollContainerRef.current) return

        const { scrollLeft, scrollWidth, clientWidth } = scrollContainerRef.current
        setShowLeftArrow(scrollLeft > 0)
        setShowRightArrow(scrollLeft < scrollWidth - clientWidth - 10) // Small buffer
    }

    // Scroll left
    const scrollLeft = () => {
        if (!scrollContainerRef.current) return
        scrollContainerRef.current.scrollBy({ left: -300, behavior: 'smooth' })
    }

    // Scroll right
    const scrollRight = () => {
        if (!scrollContainerRef.current) return
        scrollContainerRef.current.scrollBy({ left: 300, behavior: 'smooth' })
    }

    return (
        <div className="mb-12 px-4">
            <div className="flex justify-between items-center mb-4">
                <h2 className="text-2xl font-semibold text-white">{title}</h2>
                <a
                    href={viewAllLink}
                    className="text-blue-400 hover:text-blue-300 text-sm font-medium transition"
                >
                    Ver todo
                </a>
            </div>

            <div className="relative">
                {/* Left scroll button */}
                {showLeftArrow && (
                    <button
                        onClick={scrollLeft}
                        className="absolute cursor-pointer transition left-0 top-1/2 -translate-y-1/2 -ml-4 z-10 bg-gray-800/80 hover:bg-gray-700 w-10 h-10 rounded-full flex items-center justify-center text-white shadow-lg"
                        aria-label="Scroll left"
                    >
                        <ChevronLeft size={24} />
                    </button>
                )}

                {/* Scrollable container */}
                <div
                    ref={scrollContainerRef}
                    className="flex overflow-x-auto snap-x snap-mandatory scrollbar-hide py-6"
                    onScroll={handleScroll}
                >
                    {modpacks.length > 0 ? (
                        modpacks.map((modpack, index) => (
                            <div
                                key={modpack.id || index}
                                className="snap-start scroll-ml-4 flex-shrink-0 w-80 mr-4 first:ml-0"
                            >
                                <ModpackCard modpack={modpack} href={'/prelaunch/THIS_SHOULD_BE_UNIQUE_UUID'} />
                            </div>
                        ))
                    ) : (
                        // Placeholder items
                        Array.from({ length: 5 }).map((_, index) => (
                            <div
                                key={index}
                                className="snap-start scroll-ml-4 flex-shrink-0 w-80 mr-4 first:ml-0"
                            >
                                <ModpackCard
                                    modpack={{
                                        id: `placeholder-${index}`,
                                        title: `Modpack ${index + 1}`,
                                        imageUrl: "/api/placeholder/400/225"
                                    }}
                                    href={href}
                                />
                            </div>
                        ))
                    )}
                </div>

                {/* Right scroll button */}
                {showRightArrow && (
                    <button
                        onClick={scrollRight}
                        className="absolute cursor-pointer transition right-0 top-1/2 -translate-y-1/2 -mr-4 z-10 bg-gray-800/80 hover:bg-gray-700 w-10 h-10 rounded-full flex items-center justify-center text-white shadow-lg"
                        aria-label="Scroll right"
                    >
                        <ChevronRight size={24} />
                    </button>
                )}
            </div>
        </div>
    )
}