#!/usr/bin/env ruby
# inject-hook.rb — Adds SwiftVMHook.swift to an Xcode project target automatically.
# Usage: ruby inject-hook.rb <path/to/Project.xcodeproj> <target_name> <path/to/SwiftVMHook.swift>

require 'xcodeproj'

xcodeproj_path = ARGV[0]
target_name    = ARGV[1]
hook_file      = ARGV[2]

unless xcodeproj_path && target_name && hook_file
  $stderr.puts "Usage: ruby inject-hook.rb <Project.xcodeproj> <target> <SwiftVMHook.swift>"
  exit 1
end

unless File.exist?(hook_file)
  $stderr.puts "Hook file not found: #{hook_file}"
  exit 1
end

project = Xcodeproj::Project.open(xcodeproj_path)
target = project.targets.find { |t| t.name == target_name }

unless target
  available = project.targets.map(&:name).join(", ")
  $stderr.puts "Target '#{target_name}' not found. Available: #{available}"
  exit 1
end

# Check if already added
hook_name = File.basename(hook_file)
already = target.source_build_phase.files.any? { |f| f.file_ref&.path&.end_with?(hook_name) }
if already
  puts "#{hook_name} already in target '#{target_name}' — skipping"
  exit 0
end

# Find or create the App group (or root group)
# Walk up from the hook file to find the relative path from the project
project_dir = File.dirname(xcodeproj_path)
hook_abs = File.expand_path(hook_file)
hook_rel = hook_abs.sub("#{project_dir}/", "")

# Find existing group that matches the parent directory
parent_dir = File.dirname(hook_rel)
group = project.main_group

parent_dir.split("/").each do |component|
  child = group.children.find { |c| c.respond_to?(:path) && c.path == component }
  child ||= group.children.find { |c| c.respond_to?(:name) && c.name == component }
  if child && child.is_a?(Xcodeproj::Project::Object::PBXGroup)
    group = child
  else
    # Create the group if it doesn't exist
    group = group.new_group(component, component)
  end
end

# Add file reference
file_ref = group.new_reference(hook_name)
if hook_name.end_with?(".m")
  file_ref.last_known_file_type = "sourcecode.c.objc"
else
  file_ref.last_known_file_type = "sourcecode.swift"
end

# Add to target's compile sources
target.source_build_phase.add_file_reference(file_ref)

project.save
puts "✓ Added #{hook_name} to target '#{target_name}' in #{File.basename(xcodeproj_path)}"
