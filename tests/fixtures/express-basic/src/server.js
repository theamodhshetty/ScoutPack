import express from "express";
const cookieParser = require("cookie-parser");

export class AuthService {
  verify(token) {
    return token === "demo";
  }
}

export function requireAuth(req, res, next) {
  const token = req.cookies.session;
  if (!token) {
    return res.redirect("/login");
  }
  return next();
}

export const loginUser = async (req, res) => {
  res.cookie("session", "demo");
  return res.redirect("/dashboard");
};

const app = express();
app.use(cookieParser());
app.get("/auth/session", requireAuth);
app.post("/login", loginUser);

export default app;
