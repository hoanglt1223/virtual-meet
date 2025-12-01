import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Search, Play, Plus, FolderOpen } from "lucide-react";
import type { MediaFile } from "@/types";

export default function MediaLibrary() {
  const [searchTerm, setSearchTerm] = useState("");
  const [selectedType, setSelectedType] = useState<"all" | "video" | "audio">("all");

  // Placeholder data - in real app this would come from Tauri commands
  const mockMediaFiles: MediaFile[] = [];

  const filteredFiles = mockMediaFiles.filter((file) => {
    const matchesSearch = file.name.toLowerCase().includes(searchTerm.toLowerCase());
    const matchesType = selectedType === "all" || file.type === selectedType;
    return matchesSearch && matchesType;
  });

  return (
    <div className="space-y-6">
      {/* Search and Filter Bar */}
      <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted-foreground h-4 w-4" />
          <Input
            placeholder="Search media files..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="pl-10"
          />
        </div>

        <div className="flex gap-2">
          <Button
            variant={selectedType === "all" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("all")}
          >
            All
          </Button>
          <Button
            variant={selectedType === "video" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("video")}
          >
            Videos
          </Button>
          <Button
            variant={selectedType === "audio" ? "default" : "outline"}
            size="sm"
            onClick={() => setSelectedType("audio")}
          >
            Audio
          </Button>
        </div>

        <Button variant="outline">
          <FolderOpen className="h-4 w-4 mr-2" />
          Add Folder
        </Button>
      </div>

      {/* Media Grid */}
      <Card>
        <CardHeader>
          <CardTitle>Media Files ({filteredFiles.length})</CardTitle>
        </CardHeader>
        <CardContent>
          {filteredFiles.length === 0 ? (
            <div className="text-center py-12">
              <div className="text-muted-foreground mb-4">
                {searchTerm || selectedType !== "all"
                  ? "No media files match your filters"
                  : "No media files found"}
              </div>
              <Button>
                <Plus className="h-4 w-4 mr-2" />
                Add Media Folder
              </Button>
            </div>
          ) : (
            <div className="grid gap-4 md:grid-cols-3 lg:grid-cols-4">
              {filteredFiles.map((file) => (
                <div
                  key={file.id}
                  className="border rounded-lg overflow-hidden hover:shadow-md transition-shadow cursor-pointer"
                >
                  <div className="aspect-video bg-muted flex items-center justify-center">
                    {file.type === "video" ? (
                      file.thumbnailPath ? (
                        <img
                          src={file.thumbnailPath}
                          alt={file.name}
                          className="w-full h-full object-cover"
                        />
                      ) : (
                        <div className="text-4xl text-muted-foreground">▶</div>
                      )
                    ) : (
                      <div className="text-4xl text-muted-foreground">♪</div>
                    )}
                  </div>
                  <div className="p-3">
                    <div className="font-medium text-sm truncate mb-1">
                      {file.name}
                    </div>
                    <div className="flex items-center justify-between mb-2">
                      <Badge variant="secondary" className="text-xs">
                        {file.type}
                      </Badge>
                      {file.duration && (
                        <span className="text-xs text-muted-foreground">
                          {Math.floor(file.duration / 60)}:{(file.duration % 60).toString().padStart(2, '0')}
                        </span>
                      )}
                    </div>
                    <div className="flex gap-1">
                      <Button size="sm" variant="outline" className="flex-1">
                        <Play className="h-3 w-3 mr-1" />
                        Set as {file.type}
                      </Button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}