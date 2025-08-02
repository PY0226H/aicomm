#!/bin/bash

echo "🚀 开始格式化 Rust 代码..."

for dir in chat chatapp/src-tauri; do
    if [ -d "$dir" ]; then
        echo "📁 格式化 $dir 目录..."
        (cd "$dir" && cargo fmt --all)
        if [ $? -eq 0 ]; then
            echo "✅ $dir 格式化完成"
        else
            echo "❌ $dir 格式化失败"
            exit 1
        fi
    else
        echo "⚠️  目录 $dir 不存在，跳过"
    fi
done

echo "🎉 所有目录格式化完成！"
