pub const VERIFY_YOUR_EMAIL_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>Please Verify Your Email</title>
  <style>
    body {
      font-family: 'Helvetica Neue', Helvetica, Arial, sans-serif;
      background-color: #f6f6f6;
      margin: 0;
      padding: 0;
    }
    .container {
      max-width: 600px;
      margin: 40px auto;
      background: #fff;
      padding: 20px;
      border-radius: 8px;
      box-shadow: 0 2px 3px rgba(0,0,0,0.1);
    }
    h1 {
      color: #333;
      font-size: 24px;
    }
    p {
      color: #555;
      line-height: 1.5;
    }
    .button {
      display: inline-block;
      padding: 12px 24px;
      margin: 20px 0;
      background-color: #007BFF;
      color: #fff;
      text-decoration: none;
      border-radius: 4px;
    }
    .footer {
      font-size: 12px;
      color: #999;
      text-align: center;
      margin-top: 20px;
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>Please Verify Your Email</h1>
    <p>We're glad you've joined us! Please click the button below to verify your email address:</p>
    <a href="$1" class="button">Verify Your Email</a>
    <p>This verification link is valid until $2. If you did not request this, please ignore this message.</p>
    <div class="footer">
      &copy; 2023 Your Company. All rights reserved.
    </div>
  </div>
</body>
</html>
"#;
