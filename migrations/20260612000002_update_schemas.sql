-- Migration: Update User and Task table columns to match specifications

-- 1. Update users table
ALTER TABLE users RENAME COLUMN password_hash TO hashed_password;
ALTER TABLE users ADD COLUMN IF NOT EXISTS full_name VARCHAR(100) NOT NULL DEFAULT 'System User';
ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();

-- 2. Update tasks table
ALTER TABLE tasks RENAME COLUMN assigned_to TO assigned_to_id;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS created_by_id UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN IF NOT EXISTS updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();
